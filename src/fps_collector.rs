//! FPS counter via Windows Graphics Capture — no admin required.
//!
//! Tracks the foreground window.  When focus changes we restart the capture
//! session automatically.  Frame pixels are *never processed* — we allocate a
//! 2×2 buffer and immediately discard every frame, so the GPU copy overhead
//! is essentially zero.  The yellow capture border is suppressed on Win 11
//! via `IsBorderRequired = false` (IGraphicsCaptureSession3).

use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use crate::models::FpsSnapshot;

pub fn start(fps: Arc<RwLock<FpsSnapshot>>) {
    thread::Builder::new()
        .name("fps-wgc".into())
        .spawn(move || {
            if let Err(e) = run(fps) {
                log::warn!("WGC FPS collector exited: {e:#}");
            }
        })
        .expect("failed to spawn fps thread");
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(windows)]
fn run(shared: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
    use windows::{
        core::{factory, IInspectable, Interface, PWSTR},
        Foundation::TypedEventHandler,
        Graphics::{
            Capture::{
                Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
                IGraphicsCaptureSession3,
            },
            DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
            SizeInt32,
        },
        Win32::{
            Foundation::HWND,
            Graphics::{
                Direct3D::D3D_DRIVER_TYPE_HARDWARE,
                Direct3D11::{
                    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
                    ID3D11Device,
                },
                Dxgi::IDXGIDevice,
            },
            System::WinRT::{
                Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
                Graphics::Capture::IGraphicsCaptureItemInterop,
            },
            UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW},
        },
    };

    // One D3D11 device shared for the lifetime of this thread.
    let d3d = create_d3d_device()?;

    // State for the active capture session — dropping it stops the session.
    struct Session {
        _wgc: GraphicsCaptureSession,
        _pool: Direct3D11CaptureFramePool,
        _token: windows::Foundation::EventRegistrationToken,
        frames: Arc<AtomicU32>,
        title: String,
    }

    let mut active: Option<Session> = None;
    let mut last_hwnd = HWND::default();
    let mut fps_tick = Instant::now();

    loop {
        thread::sleep(Duration::from_secs(1));

        let fg = unsafe { GetForegroundWindow() };

        if fg != last_hwnd {
            active = None; // drop → stops old session
            last_hwnd = fg;

            if fg.0 != 0 {
                let frames = Arc::new(AtomicU32::new(0));
                let frames_cb = Arc::clone(&frames);
                let title = window_title(fg);

                let result: windows::core::Result<Session> = (|| {
                    let interop: IGraphicsCaptureItemInterop =
                        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
                    let item: GraphicsCaptureItem =
                        unsafe { interop.CreateForWindow(fg) }?;

                    // 2×2 pixel buffer — content is discarded, only arrival matters.
                    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                        &d3d,
                        DirectXPixelFormat::B8G8R8A8UIntNormalized,
                        1,
                        SizeInt32 { Width: 2, Height: 2 },
                    )?;

                    let token = pool.FrameArrived(&TypedEventHandler::<
                        Direct3D11CaptureFramePool,
                        IInspectable,
                    >::new(
                        move |p: &Option<Direct3D11CaptureFramePool>,
                              _: &Option<IInspectable>| {
                            if let Some(p) = p {
                                // Dequeue immediately — zero pixel work.
                                let _ = p.TryGetNextFrame().map(|f| f.Close());
                            }
                            frames_cb.fetch_add(1, Ordering::Relaxed);
                            Ok(())
                        },
                    ))?;

                    let session = pool.CreateCaptureSession(&item)?;

                    // Suppress yellow capture border on Windows 11.
                    if let Ok(s3) = session.cast::<IGraphicsCaptureSession3>() {
                        let _ = s3.SetIsBorderRequired(false);
                    }
                    session.StartCapture()?;

                    Ok(Session {
                        _wgc: session,
                        _pool: pool,
                        _token: token,
                        frames,
                        title,
                    })
                })();

                match result {
                    Ok(s) => {
                        active = Some(s);
                        fps_tick = Instant::now();
                    }
                    Err(e) => log::debug!("WGC: skipping window: {e}"),
                }
            }
        }

        // Compute FPS from atomic counter reset.
        let elapsed = fps_tick.elapsed();
        if elapsed >= Duration::from_millis(900) {
            match &active {
                Some(s) => {
                    let count = s.frames.swap(0, Ordering::Relaxed);
                    let fps = count as f32 / elapsed.as_secs_f32();
                    fps_tick = Instant::now();
                    if let Ok(mut g) = shared.write() {
                        g.fps = fps;
                        g.window_title = s.title.clone();
                        g.active = fps > 1.0;
                    }
                }
                None => {
                    if let Ok(mut g) = shared.write() {
                        g.fps = 0.0;
                        g.active = false;
                        g.window_title.clear();
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
fn create_d3d_device() -> anyhow::Result<IDirect3DDevice> {
    use windows::{
        Graphics::DirectX::Direct3D11::IDirect3DDevice,
        Win32::{
            Graphics::{
                Direct3D::D3D_DRIVER_TYPE_HARDWARE,
                Direct3D11::{
                    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
                    ID3D11Device,
                },
                Dxgi::IDXGIDevice,
            },
            System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
        },
    };

    let mut raw: Option<ID3D11Device> = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut raw),
            None,
            None,
        )?;
    }
    let d3d = raw.unwrap();
    let dxgi: IDXGIDevice = d3d.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
    Ok(inspectable.cast::<IDirect3DDevice>()?)
}

#[cfg(windows)]
fn window_title(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::{core::PWSTR, Win32::UI::WindowsAndMessaging::GetWindowTextW};
    let mut buf = vec![0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, PWSTR(buf.as_mut_ptr()), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

// ── Non-Windows stub ──────────────────────────────────────────────────────────

#[cfg(not(windows))]
fn run(_shared: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
    Ok(())
}

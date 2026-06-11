//! FPS counter — ETW-based when admin (counts every DXGI Present_Stop), WGC
//! fallback when not (counts DWM-composed frames, capped to monitor refresh).

use std::sync::{Arc, RwLock};
use std::thread;

use crate::models::FpsSnapshot;

pub fn start(fps: Arc<RwLock<FpsSnapshot>>) {
    thread::Builder::new()
        .name("fps-collector".into())
        .spawn(move || {
            if let Err(e) = inner::run(fps) {
                log::warn!("FPS collector exited: {e:#}");
            }
        })
        .expect("failed to spawn fps thread");
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(windows)]
mod inner {
    use std::{
        collections::HashMap,
        ffi::c_void,
        sync::{Arc, RwLock},
        thread,
        time::{Duration, Instant},
    };

    use parking_lot::Mutex;
    use windows::{
        core::{GUID, PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, WIN32_ERROR},
            System::{
                Diagnostics::Etw::{
                    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace,
                    StartTraceW, CONTROLTRACE_HANDLE, EVENT_RECORD, EVENT_TRACE_CONTROL,
                    EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES, PROCESSTRACE_HANDLE,
                },
                Threading::{
                    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                    PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
        },
    };

    use crate::models::FpsSnapshot;

    // ── ETW constants ─────────────────────────────────────────────────────────

    const WNODE_FLAG_TRACED_GUID: u32 = 0x0002_0000;
    const EVENT_TRACE_REAL_TIME_MODE: u32 = 0x0000_0100;
    const TRACE_LEVEL_INFORMATION: u8 = 4;
    const PROCESS_TRACE_MODE_REAL_TIME: u32 = 0x0000_0100;
    const PROCESS_TRACE_MODE_EVENT_RECORD: u32 = 0x1000_0000;

    // ── Provider GUIDs ────────────────────────────────────────────────────────

    // Microsoft-Windows-DXGI  {CA11C036-0102-4A2D-A6AD-F03CFED5D3C9}
    const DXGI_GUID: GUID = GUID::from_values(
        0xCA11_C036, 0x0102, 0x4A2D,
        [0xA6, 0xAD, 0xF0, 0x3C, 0xFE, 0xD5, 0xD3, 0xC9],
    );
    // Microsoft-Windows-DxgKrnl  {802EC45A-1E99-4B83-9920-87C98277BA9D}
    const DXGKRNL_GUID: GUID = GUID::from_values(
        0x802E_C45A, 0x1E99, 0x4B83,
        [0x99, 0x20, 0x87, 0xC9, 0x82, 0x77, 0xBA, 0x9D],
    );


    // ── Present tracking ──────────────────────────────────────────────────────

    struct PresentEntry {
        name: String,
        total: u64,
        last_total: u64,
        last_sample: Instant,
        last_present: Instant,
    }

    struct EtwState {
        presents: Mutex<HashMap<u32, PresentEntry>>,
    }

    // Called on the ProcessTrace thread — must be fast.
    unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
        if record.is_null() {
            return;
        }
        let r = &*record;

        let pid = r.EventHeader.ProcessId;
        if pid == 0 || pid == 4 {
            return;
        }

        let opcode = r.EventHeader.EventDescriptor.Opcode;
        let task   = r.EventHeader.EventDescriptor.Task;

        // Accept two classes of "one present = one count" events:
        //
        // 1. DXGI Present_Stop (task 9, opcode 2) — fires for DWM-composited games.
        //    Games using the MPO/IndependentFlip path do NOT fire this event.
        //
        // 2. DxgKrnl flip events (opcode 0, info) — fire for games using
        //    Independent Flip / MPO direct presentation. Per-frame, game's PID.
        //      task  3 = Flip
        //      task 17 = MMIOFlip
        //      task143 = FlipMultiPlaneOverlay
        //      task144 = MMIOFlipMultiPlaneOverlay
        //      task151 = IndependentFlip (modern DX11/DX12 borderless games)
        let is_present = if r.EventHeader.ProviderId == DXGI_GUID {
            opcode == 2 && task == 9
        } else if r.EventHeader.ProviderId == DXGKRNL_GUID {
            opcode == 0 && matches!(task, 3 | 17 | 143 | 144 | 151)
        } else {
            false
        };

        if !is_present {
            return;
        }

        let state = &*(r.UserContext as *const EtwState);
        let now = Instant::now();
        let mut map = state.presents.lock();
        map.entry(pid)
            .and_modify(|e| {
                e.total += 1;
                e.last_present = now;
            })
            .or_insert_with(|| PresentEntry {
                name: proc_name(pid),
                total: 1,
                last_total: 0,
                last_sample: now,
                last_present: now,
            });
    }

    fn proc_name(pid: u32) -> String {
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return pid.to_string();
            };
            let mut buf = vec![0u16; 260];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .is_ok();
            let _ = CloseHandle(handle);
            if !ok || size == 0 {
                return pid.to_string();
            }
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string()
        }
    }

    // ── ETW session helpers ───────────────────────────────────────────────────

    fn session_name_wide() -> Vec<u16> {
        "BSMonitorFps\0".encode_utf16().collect()
    }

    fn kill_session() {
        let name = session_name_wide();
        let sz = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + name.len() * 2;
        let mut buf = vec![0u8; sz];
        let p = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        unsafe {
            (*p).Wnode.BufferSize = sz as u32;
            (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            let _ = ControlTraceW(
                CONTROLTRACE_HANDLE::default(),
                PCWSTR(name.as_ptr()),
                p,
                EVENT_TRACE_CONTROL(1), // EVENT_TRACE_CONTROL_STOP
            );
        }
    }

    fn create_session() -> anyhow::Result<CONTROLTRACE_HANDLE> {
        kill_session();

        let name = session_name_wide();
        let name_bytes: Vec<u8> = name.iter().flat_map(|w| w.to_le_bytes()).collect();
        let sz = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + name_bytes.len();
        let mut buf = vec![0u8; sz];
        let p = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        unsafe {
            (*p).Wnode.BufferSize = sz as u32;
            (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            (*p).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*p).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
            std::ptr::copy_nonoverlapping(
                name_bytes.as_ptr(),
                buf.as_mut_ptr()
                    .add(std::mem::size_of::<EVENT_TRACE_PROPERTIES>()),
                name_bytes.len(),
            );
        }
        let mut handle = CONTROLTRACE_HANDLE::default();
        let rc = unsafe { StartTraceW(&mut handle, PCWSTR(name.as_ptr()), p) };
        if rc != WIN32_ERROR(0) {
            anyhow::bail!("StartTraceW error {:?}", rc);
        }
        Ok(handle)
    }

    fn enable_providers(session: CONTROLTRACE_HANDLE) {
        unsafe {
            // DXGI — all events; callback filters to Present_Stop (task 9, opcode 2)
            let _ = EnableTraceEx2(
                session,
                &DXGI_GUID,
                1, // EVENT_CONTROL_CODE_ENABLE_PROVIDER
                TRACE_LEVEL_INFORMATION,
                u64::MAX,
                0,
                0,
                None,
            );
            // DxgKrnl — "Present" keyword (0x8000000) enables flip events.
            // Flip/MMIOFlip/IndependentFlip events all carry this keyword bit.
            // Callback filters to flip tasks (3,17,143,144,151) with opcode 0.
            let _ = EnableTraceEx2(
                session,
                &DXGKRNL_GUID,
                1, // EVENT_CONTROL_CODE_ENABLE_PROVIDER
                TRACE_LEVEL_INFORMATION,
                0x0800_0000u64,
                0,
                0,
                None,
            );
        }
    }

    // ── ETW run loop ──────────────────────────────────────────────────────────

    fn run_etw(shared: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
        let session = create_session()?;

        let state = Arc::new(EtwState {
            presents: Mutex::new(HashMap::new()),
        });
        // Leak an Arc ref for the callback's UserContext; reclaimed when ProcessTrace exits.
        let ctx = Arc::into_raw(Arc::clone(&state)) as *mut c_void;

        enable_providers(session);

        let trace: PROCESSTRACE_HANDLE = {
            let mut name = session_name_wide();
            let mut lf = EVENT_TRACE_LOGFILEW::default();
            unsafe {
                lf.LoggerName = PWSTR(name.as_mut_ptr());
                lf.Anonymous1.ProcessTraceMode =
                    PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
                lf.Anonymous2.EventRecordCallback = Some(on_event);
                lf.Context = ctx;
                OpenTraceW(&mut lf)
            }
        };
        // INVALID_PROCESSTRACE_HANDLE = 0xFFFF_FFFF_FFFF_FFFF
        if trace.Value == u64::MAX {
            unsafe {
                drop(Arc::from_raw(ctx as *const EtwState));
            }
            anyhow::bail!("OpenTraceW failed");
        }

        // ctx is *mut c_void (not Send); convert to usize for the thread boundary.
        // trace is PROCESSTRACE_HANDLE { Value: u64 } which is Send.
        let ctx_addr = ctx as usize;
        thread::Builder::new()
            .name("fps-etw-pt".into())
            .spawn(move || unsafe {
                let _ = ProcessTrace(&[trace], None, None);
                let _ = CloseTrace(trace);
                drop(Arc::from_raw(ctx_addr as *const EtwState));
            })
            .expect("spawn fps-etw-pt");

        // Publish the top FPS process every second.
        loop {
            thread::sleep(Duration::from_secs(1));
            let now = Instant::now();

            let (top_fps, top_name) = {
                let mut map = state.presents.lock();

                let mut best: f32 = 0.0;
                let mut best_name = String::new();

                for entry in map.values_mut() {
                    let elapsed = now.duration_since(entry.last_sample).as_secs_f32();
                    if elapsed < 0.1 {
                        continue;
                    }
                    let delta = entry.total.saturating_sub(entry.last_total);
                    let fps = (delta as f32 / elapsed).min(9999.0);
                    entry.last_total = entry.total;
                    entry.last_sample = now;
                    if fps > best {
                        best = fps;
                        best_name = entry.name.clone();
                    }
                }

                // Evict processes quiet for 5+ seconds
                map.retain(|_, e| now.duration_since(e.last_present) < Duration::from_secs(5));

                (best, best_name)
            };

            if let Ok(mut g) = shared.write() {
                g.fps = top_fps;
                g.active = top_fps > 1.0;
                g.window_title = if top_fps > 1.0 { top_name } else { String::new() };
            }
        }
    }

    // ── WGC fallback ──────────────────────────────────────────────────────────

    use std::sync::atomic::{AtomicU32, Ordering};
    use windows::{
        core::{factory, IInspectable, Interface},
        Foundation::TypedEventHandler,
        Graphics::{
            Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
            DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
            SizeInt32,
        },
        Win32::{
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

    struct WgcSession {
        _wgc: GraphicsCaptureSession,
        _pool: Direct3D11CaptureFramePool,
        _token: windows::Foundation::EventRegistrationToken,
        frames: Arc<AtomicU32>,
        title: String,
    }

    fn run_wgc(shared: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
        let d3d = wgc_device()?;
        let mut active: Option<WgcSession> = None;
        let mut last_hwnd = windows::Win32::Foundation::HWND::default();
        let mut fps_tick = Instant::now();

        loop {
            thread::sleep(Duration::from_secs(1));

            let fg = unsafe { GetForegroundWindow() };
            if fg != last_hwnd {
                active = None;
                last_hwnd = fg;
                if fg.0 != 0 {
                    match wgc_start(&d3d, fg) {
                        Ok(s) => {
                            active = Some(s);
                            fps_tick = Instant::now();
                        }
                        Err(e) => log::debug!("WGC skip: {e}"),
                    }
                }
            }

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

    fn wgc_start(
        d3d: &IDirect3DDevice,
        hwnd: windows::Win32::Foundation::HWND,
    ) -> windows::core::Result<WgcSession> {
        let interop: IGraphicsCaptureItemInterop =
            factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd) }?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            d3d,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            SizeInt32 { Width: 2, Height: 2 },
        )?;
        let frames = Arc::new(AtomicU32::new(0));
        let frames_cb = Arc::clone(&frames);
        let token = pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |p: &Option<Direct3D11CaptureFramePool>, _: &Option<IInspectable>| {
                    if let Some(p) = p {
                        let _ = p.TryGetNextFrame().map(|f| f.Close());
                    }
                    frames_cb.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                },
            ),
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        let _ = session.SetIsBorderRequired(false);
        session.StartCapture()?;
        Ok(WgcSession {
            _wgc: session,
            _pool: pool,
            _token: token,
            frames,
            title: wgc_title(hwnd),
        })
    }

    fn wgc_device() -> anyhow::Result<IDirect3DDevice> {
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
        let dxgi: IDXGIDevice = raw.unwrap().cast()?;
        let ins = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
        Ok(ins.cast::<IDirect3DDevice>()?)
    }

    fn wgc_title(hwnd: windows::Win32::Foundation::HWND) -> String {
        let mut buf = vec![0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..len.max(0) as usize])
    }

    // ── Entry point ───────────────────────────────────────────────────────────

    pub fn run(shared: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
        match run_etw(Arc::clone(&shared)) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!("ETW FPS unavailable ({e}), using WGC fallback");
                run_wgc(shared)
            }
        }
    }
}

// ── Non-Windows stub ──────────────────────────────────────────────────────────

#[cfg(not(windows))]
mod inner {
    use std::sync::{Arc, RwLock};
    use crate::models::FpsSnapshot;
    pub fn run(_: Arc<RwLock<FpsSnapshot>>) -> anyhow::Result<()> {
        Ok(())
    }
}

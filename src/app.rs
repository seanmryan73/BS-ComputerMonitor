//! Root application struct — owns shared state and drives the UI each frame.

use std::sync::{Arc, Mutex, RwLock};

use egui::Context;
use serde::{Deserialize, Serialize};

use crate::{
    collector, fps_collector,
    models::{FpsSnapshot, MetricHistory, SystemSnapshot, HISTORY_LEN},
    theme::Theme,
    ui,
};

fn config_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("BSComputerMonitor").join("config.json")
}

/// Persisted user config — card visibility + window opacity + display mode.
#[derive(Clone, Serialize, Deserialize)]
pub struct CardVisibility {
    pub show_fps:  bool,
    pub show_gpu:  bool,
    pub show_net:  bool,
    pub show_disk: bool,
    pub show_temp: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default = "default_compact_font_size")]
    pub compact_font_size: f32,
    #[serde(skip)]
    pub always_on_top: bool,
    #[serde(skip)]
    pub passthrough_mode: bool,
}

fn default_opacity() -> f32 { 1.0 }
fn default_compact_font_size() -> f32 { 22.0 }

/// Per-card collapse animation state for the 5 optional cards [fps, gpu, net, disk, temp].
pub struct CardAnim {
    /// Visual scale: 0.0 = fully collapsed/hidden, 1.0 = fully shown.
    pub scale:  [f32; 5],
    /// Last measured card content height (px, excluding item_spacing) — used as
    /// the collapse target so surrounding cards slide smoothly as this one shrinks.
    pub height: [f32; 5],
}

impl CardAnim {
    fn new(vis: &CardVisibility) -> Self {
        Self {
            scale: [
                if vis.show_fps  { 1.0 } else { 0.0 },
                if vis.show_gpu  { 1.0 } else { 0.0 },
                if vis.show_net  { 1.0 } else { 0.0 },
                if vis.show_disk { 1.0 } else { 0.0 },
                if vis.show_temp { 1.0 } else { 0.0 },
            ],
            height: [95.0; 5],
        }
    }
}

impl Default for CardVisibility {
    fn default() -> Self {
        Self {
            show_fps: true, show_gpu: true, show_net: true,
            show_disk: true, show_temp: true,
            opacity: 1.0,
            compact_mode: false,
            compact_font_size: 22.0,
            always_on_top: false,
            passthrough_mode: false,
        }
    }
}

impl CardVisibility {
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub struct MonitorApp {
    snapshot: Arc<RwLock<SystemSnapshot>>,
    pub fps: Arc<RwLock<FpsSnapshot>>,
    pub theme: Theme,

    pub hist_cpu: MetricHistory,
    pub hist_mem: MetricHistory,
    pub hist_rx: MetricHistory,
    pub hist_tx: MetricHistory,
    pub hist_gpu: MetricHistory,
    pub hist_fps: MetricHistory,
    pub hist_temp_cpu: MetricHistory,
    pub hist_temp_gpu: MetricHistory,
    pub hist_disk:     MetricHistory,

    // Previous-tick snapshots — interpolation source for silky motion
    prev_cpu:      Vec<f64>,
    prev_mem:      Vec<f64>,
    prev_rx:       Vec<f64>,
    prev_tx:       Vec<f64>,
    prev_gpu:      Vec<f64>,
    prev_fps:      Vec<f64>,
    prev_temp_cpu: Vec<f64>,
    prev_temp_gpu: Vec<f64>,
    prev_disk:     Vec<f64>,

    // Display buffers — smootherstep-interpolated between prev and hist each frame
    pub disp_cpu:      Vec<f64>,
    pub disp_mem:      Vec<f64>,
    pub disp_rx:       Vec<f64>,
    pub disp_tx:       Vec<f64>,
    pub disp_gpu:      Vec<f64>,
    pub disp_fps:      Vec<f64>,
    pub disp_temp_cpu: Vec<f64>,
    pub disp_temp_gpu: Vec<f64>,
    pub disp_disk:     Vec<f64>,

    // 0.0 just after a tick fires → 1.0 just before the next tick
    tick_phase: f32,

    last_tick: std::time::Instant,
    first_tick: bool,
    pub show_about: bool,
    pub card_vis: Arc<Mutex<CardVisibility>>,
    hwnd: Option<isize>,
    applied_opacity: f32,
    opacity_startup_frames: u8,
    prev_always_on_top: bool,
    prev_compact_mode: Option<bool>,
    prev_passthrough_mode: bool,
    pub passthrough_active: bool,
    pub prev_show_about: bool,
    pub is_elevated: bool,

    /// Per-card collapse/expand animation (height squish).
    pub card_anim: CardAnim,
    /// 0.0 = fully in normal mode, 1.0 = fully in compact mode.
    /// Drives the cross-dissolve when the user switches between modes.
    pub compact_anim: f32,
    /// Tracks which layout was rendered last frame so we can fire the window
    /// resize exactly when compact_anim crosses 0.5 (opacity == 0).
    prev_render_compact: Option<bool>,
    /// Window height saved just before entering compact mode, restored on exit.
    saved_normal_height: Option<f32>,
    /// Tracks which optional cards were shown last frame so we can snap the
    /// window height whenever a card is added or removed.
    prev_shown_cards: [bool; 5],
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::default();
        theme.apply(&cc.egui_ctx);

        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));
        collector::start(Arc::clone(&snapshot));

        let fps = Arc::new(RwLock::new(FpsSnapshot::default()));
        fps_collector::start(Arc::clone(&fps));

        let vis_init = CardVisibility::load();
        let card_anim_init    = CardAnim::new(&vis_init);
        let compact_anim_init = if vis_init.compact_mode { 1.0_f32 } else { 0.0_f32 };

        Self {
            snapshot,
            fps,
            theme,
            hist_cpu: MetricHistory::new(HISTORY_LEN),
            hist_mem: MetricHistory::new(HISTORY_LEN),
            hist_rx: MetricHistory::new(HISTORY_LEN),
            hist_tx: MetricHistory::new(HISTORY_LEN),
            hist_gpu: MetricHistory::new(HISTORY_LEN),
            hist_fps: MetricHistory::new(HISTORY_LEN),
            hist_temp_cpu: MetricHistory::new(HISTORY_LEN),
            hist_temp_gpu: MetricHistory::new(HISTORY_LEN),
            hist_disk:     MetricHistory::new(HISTORY_LEN),
            prev_cpu:      Vec::new(),
            prev_mem:      Vec::new(),
            prev_rx:       Vec::new(),
            prev_tx:       Vec::new(),
            prev_gpu:      Vec::new(),
            prev_fps:      Vec::new(),
            prev_temp_cpu: Vec::new(),
            prev_temp_gpu: Vec::new(),
            prev_disk:     Vec::new(),
            disp_cpu:      Vec::new(),
            disp_mem:      Vec::new(),
            disp_rx:       Vec::new(),
            disp_tx:       Vec::new(),
            disp_gpu:      Vec::new(),
            disp_fps:      Vec::new(),
            disp_temp_cpu: Vec::new(),
            disp_temp_gpu: Vec::new(),
            disp_disk:     Vec::new(),
            tick_phase: 0.0,
            // Subtract one interval so the first tick fires immediately on first frame
            last_tick: std::time::Instant::now()
                .checked_sub(crate::collector::INTERVAL)
                .unwrap_or_else(std::time::Instant::now),
            first_tick: true,
            show_about: false,
            card_vis: Arc::new(Mutex::new(vis_init)),
            hwnd: None,
            applied_opacity: -1.0, // force first-frame application
            opacity_startup_frames: 0,
            prev_always_on_top: false,
            prev_compact_mode: None,
            prev_passthrough_mode: false,
            passthrough_active: false,
            prev_show_about: false,
            is_elevated: check_elevated(),
            card_anim:    card_anim_init,
            compact_anim: compact_anim_init,
            // Seed to match the actual startup layout so frame 1 doesn't trigger a spurious resize.
            prev_render_compact: Some(compact_anim_init > 0.5),
            saved_normal_height: None,
            prev_shown_cards: [
                vis_init.show_fps, vis_init.show_gpu, vis_init.show_net,
                vis_init.show_disk, vis_init.show_temp,
            ],
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot, fps_snap: &FpsSnapshot) {
        let is_first = self.first_tick;
        if is_first { self.first_tick = false; }

        // Freeze current histograms as the interpolation source before overwriting.
        self.prev_cpu      = self.hist_cpu.as_vec();
        self.prev_mem      = self.hist_mem.as_vec();
        self.prev_rx       = self.hist_rx.as_vec();
        self.prev_tx       = self.hist_tx.as_vec();
        self.prev_gpu      = self.hist_gpu.as_vec();
        self.prev_fps      = self.hist_fps.as_vec();
        self.prev_temp_cpu = self.hist_temp_cpu.as_vec();
        self.prev_temp_gpu = self.hist_temp_gpu.as_vec();
        self.prev_disk     = self.hist_disk.as_vec();

        let n = if is_first { HISTORY_LEN } else { 1 };
        for _ in 0..n {
            self.hist_cpu.push(snap.cpu.total_usage);
            self.hist_mem.push(snap.memory.usage_percent());
            self.hist_rx.push(snap.network.total_rx_bps as f32);
            self.hist_tx.push(snap.network.total_tx_bps as f32);
            if let Some(u) = snap.gpu.utilization_percent {
                self.hist_gpu.push(u);
            }
            if fps_snap.active {
                self.hist_fps.push(fps_snap.fps);
            }
            if let Some(t) = snap.temps.cpu_celsius {
                self.hist_temp_cpu.push(t);
            }
            if let Some(t) = snap.temps.gpu_celsius {
                self.hist_temp_gpu.push(t);
            }
            if let Some(d) = snap.disks.first() {
                self.hist_disk.push(d.usage_percent());
            }
        }

        // On first tick align prev with curr so bars start fully drawn.
        if is_first {
            self.prev_cpu      = self.hist_cpu.as_vec();
            self.prev_mem      = self.hist_mem.as_vec();
            self.prev_rx       = self.hist_rx.as_vec();
            self.prev_tx       = self.hist_tx.as_vec();
            self.prev_gpu      = self.hist_gpu.as_vec();
            self.prev_fps      = self.hist_fps.as_vec();
            self.prev_temp_cpu = self.hist_temp_cpu.as_vec();
            self.prev_temp_gpu = self.hist_temp_gpu.as_vec();
            self.prev_disk     = self.hist_disk.as_vec();
        }

        self.tick_phase = 0.0;
    }

    fn advance_displays(&mut self, dt: f32) {
        let interval = crate::collector::INTERVAL.as_secs_f32();
        self.tick_phase = (self.tick_phase + dt / interval).min(1.0);
        let t = smootherstep(self.tick_phase) as f64;

        interp_buf(&mut self.disp_cpu,      &self.prev_cpu,      &self.hist_cpu.as_vec(),      t);
        interp_buf(&mut self.disp_mem,      &self.prev_mem,      &self.hist_mem.as_vec(),      t);
        interp_buf(&mut self.disp_rx,       &self.prev_rx,       &self.hist_rx.as_vec(),       t);
        interp_buf(&mut self.disp_tx,       &self.prev_tx,       &self.hist_tx.as_vec(),       t);
        interp_buf(&mut self.disp_gpu,      &self.prev_gpu,      &self.hist_gpu.as_vec(),      t);
        interp_buf(&mut self.disp_fps,      &self.prev_fps,      &self.hist_fps.as_vec(),      t);
        interp_buf(&mut self.disp_temp_cpu, &self.prev_temp_cpu, &self.hist_temp_cpu.as_vec(), t);
        interp_buf(&mut self.disp_temp_gpu, &self.prev_temp_gpu, &self.hist_temp_gpu.as_vec(), t);
        interp_buf(&mut self.disp_disk,     &self.prev_disk,     &self.hist_disk.as_vec(),     t);
    }

    /// Advance card-collapse and compact-mode cross-dissolve animations.
    pub fn advance_card_anims(&mut self, dt: f32, vis: &CardVisibility) {
        const CARD_SPD:    f32 = 5.0;  // 200 ms full collapse/expand
        const COMPACT_SPD: f32 = 4.0;  // 250 ms full mode cross-dissolve

        let step = dt * CARD_SPD;
        let shows = [vis.show_fps, vis.show_gpu, vis.show_net, vis.show_disk, vis.show_temp];
        for (i, &show) in shows.iter().enumerate() {
            move_toward(&mut self.card_anim.scale[i], if show { 1.0 } else { 0.0 }, step);
        }
        move_toward(
            &mut self.compact_anim,
            if vis.compact_mode { 1.0 } else { 0.0 },
            dt * COMPACT_SPD,
        );
    }
}

/// Smootherstep: Ken Perlin's C1-continuous S-curve.  Starts slow, eases
/// through the middle, arrives softly — zero first-derivative at both ends.
fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Move `val` toward `target` by at most `step`, clamping at target.
fn move_toward(val: &mut f32, target: f32, step: f32) {
    let diff = target - *val;
    if diff.abs() <= step { *val = target; } else { *val += diff.signum() * step; }
}

/// Compute the window height that fits the normal-mode card layout for the given config.
///
/// Card height estimate: spectrum(44) + header(14) + stat_line(12) + internal spacing(9)
/// + frame overhead(16) ≈ 95 px.  Item spacing between cards = 4 px.
fn normal_window_height(vis: &CardVisibility) -> f32 {
    let n_optional = [vis.show_fps, vis.show_gpu, vis.show_net, vis.show_disk, vis.show_temp]
        .iter()
        .filter(|&&x| x)
        .count();
    let n = 2 + n_optional;
    let content_h = n as f32 * 95.0 + (n.saturating_sub(1)) as f32 * 4.0;
    36.0 + 24.0 + content_h  // titlebar + panel inner_margins + content
}

/// Compute the window height that exactly fits the compact layout for the given config.
///
/// Formula: titlebar(36) + panel_margins(24) + n_cards × row_height + (n_cards−1) × item_spacing
/// where row_height = card_content(font_size+12, min 24) + frame_overhead(14).
fn compact_window_height(vis: &CardVisibility) -> f32 {
    let n_optional = [vis.show_fps, vis.show_gpu, vis.show_net, vis.show_disk, vis.show_temp]
        .iter()
        .filter(|&&x| x)
        .count();
    let n = 2 + n_optional;  // CPU + MEM always shown
    let row_h = (vis.compact_font_size + 12.0).max(24.0) + 14.0; // content + frame overhead
    let content_h = n as f32 * row_h + (n.saturating_sub(1)) as f32 * 2.0;
    36.0 + 24.0 + content_h  // titlebar + panel inner_margins + content
}

/// Interpolate display buffer element-by-element between prev and curr using
/// pre-computed eased t.  Size mismatch snaps immediately (startup guard).
fn interp_buf(disp: &mut Vec<f64>, prev: &[f64], curr: &[f64], t: f64) {
    let n = curr.len();
    if disp.len() != n {
        *disp = curr.to_vec();
        return;
    }
    for i in 0..n {
        let p = prev.get(i).copied().unwrap_or(curr[i]);
        disp[i] = p + (curr[i] - p) * t;
    }
}

#[cfg(windows)]
fn ctrl_held() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    const VK_CONTROL: i32 = 0x11;
    unsafe { (GetAsyncKeyState(VK_CONTROL) as u16) & 0x8000 != 0 }
}

#[cfg(not(windows))]
fn ctrl_held() -> bool { false }

#[cfg(windows)]
fn get_main_hwnd() -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::PCWSTR;
    let title: Vec<u16> = "BS Computer Monitor\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) };
    if hwnd.0 != 0 { Some(hwnd.0) } else { None }
}

#[cfg(not(windows))]
fn get_main_hwnd() -> Option<isize> { None }

#[cfg(windows)]
fn apply_window_opacity(hwnd: isize, opacity: f32) {
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetLayeredWindowAttributes,
        GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
    };
    let hwnd = HWND(hwnd);
    let alpha = (opacity.clamp(0.15, 1.0) * 255.0).round() as u8;
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
    }
}

#[cfg(not(windows))]
fn apply_window_opacity(_hwnd: isize, _opacity: f32) {}

#[cfg(windows)]
fn check_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        ).is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
fn check_elevated() -> bool { true }

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // 60 fps for smooth animation
        ctx.request_repaint();

        // Find the main window HWND once (FindWindowW is reliable on Windows)
        if self.hwnd.is_none() {
            self.hwnd = get_main_hwnd();
        }

        // Passthrough and pin-on-top are coupled: one implies the other.
        let passthrough_mode = self.card_vis.lock().map(|v| v.passthrough_mode).unwrap_or(false);
        if passthrough_mode != self.prev_passthrough_mode {
            if let Ok(mut v) = self.card_vis.lock() {
                v.always_on_top = passthrough_mode;
            }
        }
        self.prev_passthrough_mode = passthrough_mode;

        // Send WindowLevel command when always_on_top changes; also reset opacity state
        // because SetWindowPos (called by winit) can strip WS_EX_LAYERED.
        let always_on_top = self.card_vis.lock().map(|v| v.always_on_top).unwrap_or(false);
        if always_on_top != self.prev_always_on_top {
            self.prev_always_on_top = always_on_top;
            self.applied_opacity = -1.0;
            self.opacity_startup_frames = 0;
            let level = if always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            // Keep the settings window at the same level if it is open
            if self.show_about {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of("about_viewport"),
                    egui::ViewportCommand::WindowLevel(level),
                );
            }
        }

        let target_opacity = self.card_vis.lock().map(|v| v.opacity).unwrap_or(1.0);
        if let Some(hwnd) = self.hwnd {
            // Re-apply for the first 10 frames (handles window-show timing on startup)
            // and whenever the value actually changes.
            let startup = self.opacity_startup_frames < 10;
            let changed = (target_opacity - self.applied_opacity).abs() > 0.001;
            if startup || changed {
                apply_window_opacity(hwnd, target_opacity);
                self.applied_opacity = target_opacity;
            }
            if self.opacity_startup_frames < 10 {
                self.opacity_startup_frames += 1;
            }
        }

        // Adjust minimum window width when compact mode changes (or on first frame)
        let compact_mode = self.card_vis.lock().map(|v| v.compact_mode).unwrap_or(false);
        if self.prev_compact_mode != Some(compact_mode) {
            self.prev_compact_mode = Some(compact_mode);
            let min_w = if compact_mode { 110.0f32 } else { 200.0f32 };
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(min_w, 200.0)));
        }

        // Snap window height at the moment compact_anim crosses 0.5 — that is exactly
        // when the content opacity is 0 and the layout switches, so the resize is invisible.
        let render_compact_now = self.compact_anim > 0.5;
        if self.prev_render_compact != Some(render_compact_now) {
            // Read the previous state BEFORE updating it so we can detect direction.
            let came_from_normal = self.prev_render_compact == Some(false);
            self.prev_render_compact = Some(render_compact_now);

            let cur_w = ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.width()).unwrap_or(350.0);
            if render_compact_now {
                // Only save the normal height when we know we were actually in normal mode
                // (not on startup when compact_mode was already true).
                if came_from_normal {
                    self.saved_normal_height = Some(
                        ctx.input(|i| i.viewport().inner_rect)
                            .map(|r| r.height())
                            .unwrap_or(650.0),
                    );
                }
                let vis = self.card_vis.lock().map(|v| v.clone()).unwrap_or_default();
                let new_h = compact_window_height(&vis);
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(cur_w, new_h)));
            } else {
                // Restore the saved normal height, or compute a sensible fit for the
                // number of active cards if we never recorded a normal-mode height
                // (e.g. app was launched directly in compact mode).
                let h = self.saved_normal_height.unwrap_or_else(|| {
                    let vis = self.card_vis.lock().map(|v| v.clone()).unwrap_or_default();
                    normal_window_height(&vis)
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(cur_w, h)));
            }
        }

        // Passthrough (game overlay): armed via config, Ctrl held → temporarily interactive
        let passthrough_armed = self.card_vis.lock().map(|v| v.passthrough_mode).unwrap_or(false);
        let want_passthrough = passthrough_armed && !ctrl_held();
        if want_passthrough != self.passthrough_active {
            self.passthrough_active = want_passthrough;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(want_passthrough));
        }

        let snap = self.snapshot.read().map(|g| g.clone()).unwrap_or_default();
        let fps_snap = self.fps.read().map(|g| g.clone()).unwrap_or_default();

        // Gate history pushes to match the background collector interval
        if self.last_tick.elapsed() >= crate::collector::INTERVAL {
            self.tick_histories(&snap, &fps_snap);
            self.last_tick = std::time::Instant::now();
        }

        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.advance_displays(dt);
        let vis_snap = self.card_vis.lock().map(|v| v.clone()).unwrap_or_default();
        self.advance_card_anims(dt, &vis_snap);

        ui::draw(self, ctx, frame, &snap, &fps_snap);
    }
}

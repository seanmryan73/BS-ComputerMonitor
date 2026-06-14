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
    #[serde(default = "default_compact_font_size")]
    pub compact_font_size: f32,
    #[serde(skip)]
    pub always_on_top: bool,
    #[serde(skip)]
    pub passthrough_mode: bool,
    /// Index into the DXGI hardware adapter list for the GPU card. Persisted.
    #[serde(default)]
    pub selected_gpu_index: usize,
    /// Bandwidth cap in Megabits/sec — sets 100% on the NET fill bar and health colours.
    #[serde(default = "default_net_cap_mbps")]
    pub net_cap_mbps: f32,
}

fn default_opacity() -> f32 { 1.0 }
fn default_compact_font_size() -> f32 { 22.0 }
fn default_net_cap_mbps() -> f32 { 1000.0 }

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
            compact_font_size: 22.0,
            always_on_top: false,
            passthrough_mode: false,
            selected_gpu_index: 0,
            net_cap_mbps: 1000.0,
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
    pub hist_rx:       MetricHistory,
    pub hist_gpu:      MetricHistory,
    pub hist_fps:      MetricHistory,
    pub hist_temp_cpu: MetricHistory,
    pub hist_disk:     MetricHistory,

    last_tick: std::time::Instant,
    first_tick: bool,
    pub show_about: bool,
    pub card_vis: Arc<Mutex<CardVisibility>>,
    hwnd: Option<isize>,
    applied_opacity: f32,
    opacity_startup_frames: u8,
    prev_always_on_top: bool,
    prev_passthrough_mode: bool,
    pub passthrough_active: bool,
    pub prev_show_about: bool,
    pub is_elevated: bool,

    /// Per-card collapse/expand animation (height squish).
    pub card_anim: CardAnim,
    /// Tracks which optional cards were shown last frame so we can snap the
    /// window height whenever a card is added or removed.
    prev_shown_cards: [bool; 5],
    /// Tracks the compact font size so a window resize fires when the slider moves.
    prev_compact_font_size: f32,
    /// True on the very first frame — fires the initial window size snap.
    startup_resize_pending: bool,

    /// Session-high watermarks — reset on app restart, used for peak ticks in compact mode.
    pub peak_cpu:    f32,
    pub peak_mem:    f32,
    pub peak_gpu:    f32,
    pub peak_disk:   f32,
    pub peak_temp:   f32,
    /// Session-peak download in bytes/sec — converted to % of cap at render time.
    pub peak_net_rx: f32,

    #[cfg(windows)]
    tray: Option<crate::tray::TrayHandle>,
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::default();
        theme.apply(&cc.egui_ctx);

        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));

        let fps = Arc::new(RwLock::new(FpsSnapshot::default()));
        fps_collector::start(Arc::clone(&fps));

        let vis_init = CardVisibility::load();
        let card_anim_init  = CardAnim::new(&vis_init);
        let prev_shown_init = [
            vis_init.show_fps, vis_init.show_gpu, vis_init.show_net,
            vis_init.show_disk, vis_init.show_temp,
        ];
        let prev_font_size_init = vis_init.compact_font_size;

        // Create card_vis Arc before starting the collector so it can read the selected GPU.
        let card_vis_arc = Arc::new(Mutex::new(vis_init));
        collector::start(Arc::clone(&snapshot), Arc::clone(&card_vis_arc));

        Self {
            snapshot,
            fps,
            theme,
            hist_cpu: MetricHistory::new(HISTORY_LEN),
            hist_mem: MetricHistory::new(HISTORY_LEN),
            hist_rx:       MetricHistory::new(HISTORY_LEN),
            hist_gpu:      MetricHistory::new(HISTORY_LEN),
            hist_fps:      MetricHistory::new(HISTORY_LEN),
            hist_temp_cpu: MetricHistory::new(HISTORY_LEN),
            hist_disk:     MetricHistory::new(HISTORY_LEN),
            // Subtract one interval so the first tick fires immediately on first frame
            last_tick: std::time::Instant::now()
                .checked_sub(crate::collector::INTERVAL)
                .unwrap_or_else(std::time::Instant::now),
            first_tick: true,
            show_about: false,
            card_vis: card_vis_arc,
            hwnd: None,
            applied_opacity: -1.0, // force first-frame application
            opacity_startup_frames: 0,
            prev_always_on_top: false,
            prev_passthrough_mode: false,
            passthrough_active: false,
            prev_show_about: false,
            is_elevated: check_elevated(),
            card_anim: card_anim_init,
            prev_shown_cards: prev_shown_init,
            prev_compact_font_size: prev_font_size_init,
            startup_resize_pending: true,
            peak_cpu:    0.0,
            peak_mem:    0.0,
            peak_gpu:    0.0,
            peak_disk:   0.0,
            peak_temp:   0.0,
            peak_net_rx: 0.0,
            #[cfg(windows)]
            tray: crate::tray::TrayHandle::build(),
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot, fps_snap: &FpsSnapshot) {
        let is_first = self.first_tick;
        if is_first { self.first_tick = false; }

        let n = if is_first { HISTORY_LEN } else { 1 };
        for _ in 0..n {
            self.hist_cpu.push(snap.cpu.total_usage);
            self.hist_mem.push(snap.memory.usage_percent());
            self.hist_rx.push(snap.network.total_rx_bps as f32);
            if let Some(u) = snap.gpu.utilization_percent {
                self.hist_gpu.push(u);
            }
            if fps_snap.active {
                self.hist_fps.push(fps_snap.fps);
            }
            if let Some(t) = snap.temps.cpu_celsius {
                self.hist_temp_cpu.push(t);
            }
            if let Some(d) = snap.disks.first() {
                self.hist_disk.push(d.usage_percent());
            }
        }

        // Update session peaks
        self.peak_cpu    = self.peak_cpu.max(snap.cpu.total_usage);
        self.peak_mem    = self.peak_mem.max(snap.memory.usage_percent());
        self.peak_net_rx = self.peak_net_rx.max(snap.network.total_rx_bps as f32);
        if let Some(u) = snap.gpu.utilization_percent { self.peak_gpu  = self.peak_gpu.max(u); }
        if let Some(d) = snap.disks.first()           { self.peak_disk = self.peak_disk.max(d.usage_percent()); }
        if let Some(t) = snap.temps.cpu_celsius       { self.peak_temp = self.peak_temp.max(t.clamp(0.0, 100.0)); }
    }

    /// Advance card-collapse animations.
    pub fn advance_card_anims(&mut self, dt: f32, vis: &CardVisibility) {
        const CARD_SPD: f32 = 5.0;  // 200 ms full collapse/expand
        let step = dt * CARD_SPD;
        let shows = [vis.show_fps, vis.show_gpu, vis.show_net, vis.show_disk, vis.show_temp];
        for (i, &show) in shows.iter().enumerate() {
            move_toward(&mut self.card_anim.scale[i], if show { 1.0 } else { 0.0 }, step);
        }
    }
}

/// Move `val` toward `target` by at most `step`, clamping at target.
fn move_toward(val: &mut f32, target: f32, step: f32) {
    let diff = target - *val;
    if diff.abs() <= step { *val = target; } else { *val += diff.signum() * step; }
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
    // content height: value text + sub-label + fill bar; frame overhead: inner_margin(10)+outer(4)=14
    let row_h = (vis.compact_font_size + 14.0).max(44.0) + 14.0;
    let content_h = n as f32 * row_h + (n.saturating_sub(1)) as f32 * 2.0;
    36.0 + 24.0 + content_h  // titlebar + panel inner_margins + content
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

        // Snapshot card visibility once — used by all resize checks and advance_card_anims.
        let vis_snap = self.card_vis.lock().map(|v| v.clone()).unwrap_or_default();

        // First frame: set min size and snap window to compact height.
        if self.startup_resize_pending {
            self.startup_resize_pending = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(110.0, 200.0)));
            let cur_w = ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.width()).unwrap_or(350.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(cur_w, compact_window_height(&vis_snap)),
            ));
        }

        // Snap height when individual cards are toggled.
        let cur_shown = [
            vis_snap.show_fps, vis_snap.show_gpu, vis_snap.show_net,
            vis_snap.show_disk, vis_snap.show_temp,
        ];
        if cur_shown != self.prev_shown_cards {
            self.prev_shown_cards = cur_shown;
            let cur_w = ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.width()).unwrap_or(350.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(cur_w, compact_window_height(&vis_snap)),
            ));
        }

        // Snap height when the font-size slider moves.
        if (vis_snap.compact_font_size - self.prev_compact_font_size).abs() > 0.1 {
            self.prev_compact_font_size = vis_snap.compact_font_size;
            let cur_w = ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.width()).unwrap_or(350.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(cur_w, compact_window_height(&vis_snap)),
            ));
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

        // Poll tray icon events — update live tooltip + handle Exit / Show commands
        #[cfg(windows)]
        if let Some(ref tray) = self.tray {
            let tip = format!("CPU {:.0}%{}",
                snap.cpu.total_usage,
                snap.gpu.utilization_percent
                    .filter(|_| snap.gpu.available)
                    .map(|g| format!("  ·  GPU {:.0}%", g))
                    .unwrap_or_default(),
            );
            tray.set_tooltip(&tip);
            while let Ok(cmd) = tray.rx.try_recv() {
                match cmd {
                    crate::tray::TrayCmd::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    crate::tray::TrayCmd::Exit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

        // Gate history pushes to match the background collector interval
        if self.last_tick.elapsed() >= crate::collector::INTERVAL {
            self.tick_histories(&snap, &fps_snap);
            self.last_tick = std::time::Instant::now();
        }

        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.advance_card_anims(dt, &vis_snap);

        ui::draw(self, ctx, frame, &snap, &fps_snap);
    }
}

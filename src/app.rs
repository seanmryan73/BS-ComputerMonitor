//! Root application struct — owns shared state and drives the UI each frame.

use std::sync::{Arc, Mutex, RwLock};

use egui::Context;
use serde::{Deserialize, Serialize};

use crate::{
    collector, fps_collector, ping_collector,
    models::{FpsSnapshot, MetricHistory, PingSnapshot, SystemSnapshot, HISTORY_LEN},
    theme::{Theme, ThemeId},
    ui,
};

/// `None` when `APPDATA` is unset: settings then live for the session only. Never `.`, which
/// would write into the install folder (vault Rust-Desktop-Standards § checklist).
pub(crate) fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .map(|d| std::path::PathBuf::from(d).join("BCComputerMonitor").join("config.json"))
}

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes via a flushed temp file and a rename, so a full disk, a kill or a power cut leaves the
/// old file intact. `std::fs::write` truncates first. From vault Rust-Persistence-Patterns.md.
pub(crate) fn write_atomic(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }

    // Append a suffix, never `with_extension()`; pid + counter keep concurrent writers apart.
    let mut tmp_name = path.as_os_str().to_os_string();
    tmp_name.push(format!(
        ".tmp{}.{}",
        std::process::id(),
        TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let tmp = std::path::PathBuf::from(tmp_name);

    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    // rename replaces the target on Windows; no remove-then-rename fallback (it deletes the good file).
    let result = std::fs::rename(&tmp, path);
    if result.is_err() { let _ = std::fs::remove_file(&tmp); }
    result
}

/// "theme_id: \"Jizo\" -> \"Washi\"" for each top-level key that differs; for the info log line.
fn describe_changes(before: Option<&str>, after: &str) -> String {
    let parse = |s: &str| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(s).ok();
    let (Some(old), Some(new)) = (before.and_then(parse), parse(after)) else {
        return "new file".into();
    };
    let changes: Vec<String> = new
        .iter()
        .filter(|(k, v)| old.get(*k) != Some(*v))
        .map(|(k, v)| match old.get(k) {
            Some(o) => format!("{k}: {o} -> {v}"),
            None => format!("{k}: {v}"),
        })
        .collect();
    if changes.is_empty() { "no changes".into() } else { changes.join(", ") }
}

/// Persisted user config — card visibility + window opacity + display mode.
#[derive(Clone, Serialize, Deserialize)]
pub struct CardVisibility {
    pub show_fps:  bool,
    pub show_gpu:  bool,
    pub show_net:  bool,
    pub show_disk: bool,
    pub show_temp: bool,
    #[serde(default = "default_true")]
    pub show_ping: bool,
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
    #[serde(default)]
    pub theme_id: ThemeId,
    #[serde(default = "default_ping_target")]
    pub ping_target: String,
    /// Off when missing from an existing config.json, so an upgrade doesn't grow the window;
    /// a first run gets them on through `Default`.
    #[serde(default)]
    pub show_top_cpu: bool,
    #[serde(default)]
    pub show_top_mem: bool,
}

/// Number of optional cards — one animation slot each, in [`CardVisibility::shown`] order.
pub const N_OPTIONAL: usize = 8;

/// Repaint delay for ambient effects (warn/crit pulse, armed passthrough). egui subtracts a
/// predicted frame time (~16 ms) from any `request_repaint_after`, so a short delay here
/// becomes a repaint every vsync — a 33 ms pulse cost ~60 fps of full redraws.
pub const AMBIENT_REPAINT: std::time::Duration = std::time::Duration::from_millis(100);

impl CardVisibility {
    /// Visibility of the optional cards by slot:
    /// [fps, gpu, net, disk, temp, ping, top_cpu, top_mem].
    pub fn shown(&self) -> [bool; N_OPTIONAL] {
        [
            self.show_fps, self.show_gpu, self.show_net, self.show_disk,
            self.show_temp, self.show_ping, self.show_top_cpu, self.show_top_mem,
        ]
    }
}

fn default_opacity() -> f32 { 1.0 }
fn default_compact_font_size() -> f32 { 22.0 }
fn default_net_cap_mbps() -> f32 { 1000.0 }
fn default_true() -> bool { true }
fn default_ping_target() -> String { "1.1.1.1".into() }

/// Per-card collapse animation state for the optional cards, slotted as [`CardVisibility::shown`].
pub struct CardAnim {
    /// Visual scale: 0.0 = fully collapsed/hidden, 1.0 = fully shown.
    pub scale:  [f32; N_OPTIONAL],
    /// Last measured card content height (px, excluding item_spacing) — used as
    /// the collapse target so surrounding cards slide smoothly as this one shrinks.
    pub height: [f32; N_OPTIONAL],
}

impl CardAnim {
    fn new(vis: &CardVisibility) -> Self {
        Self {
            scale: vis.shown().map(|s| if s { 1.0 } else { 0.0 }),
            height: [95.0; N_OPTIONAL],
        }
    }
}

impl Default for CardVisibility {
    fn default() -> Self {
        Self {
            show_fps: true, show_gpu: true, show_net: true,
            show_disk: true, show_temp: true, show_ping: true,
            opacity: 1.0,
            compact_font_size: 22.0,
            always_on_top: false,
            passthrough_mode: false,
            selected_gpu_index: 0,
            net_cap_mbps: 1000.0,
            theme_id: ThemeId::ColdSteel,
            ping_target: "1.1.1.1".into(),
            show_top_cpu: true,
            show_top_mem: true,
        }
    }
}

impl CardVisibility {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            log::warn!("APPDATA is not set; using default settings for this session");
            return Self::default();
        };
        // A missing file is normal (first run); a parse failure means the file is
        // corrupt — warn so the user knows why their settings reset.
        let Ok(s) = std::fs::read_to_string(&path) else { return Self::default() };
        serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!("failed to parse {}: {e}; resetting to defaults", path.display());
            Self::default()
        })
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            log::warn!("APPDATA is not set; settings not saved");
            return;
        };
        let json = match serde_json::to_string_pretty(self) {
            Ok(j) => j,
            Err(e) => {
                log::warn!("could not serialise settings: {e}");
                return;
            }
        };
        let before = std::fs::read_to_string(&path).ok();
        match write_atomic(&path, &json) {
            Ok(()) => log::info!("settings saved: {}", describe_changes(before.as_deref(), &json)),
            Err(e) => log::warn!("could not save settings to {}: {e}", path.display()),
        }
    }
}

pub struct MonitorApp {
    snapshot: Arc<RwLock<SystemSnapshot>>,
    pub fps:  Arc<RwLock<FpsSnapshot>>,
    pub ping: Arc<RwLock<PingSnapshot>>,
    pub ping_target_shared: Arc<RwLock<String>>,
    pub theme: Theme,

    pub hist_cpu: MetricHistory,
    pub hist_mem: MetricHistory,
    pub hist_rx:       MetricHistory,
    pub hist_gpu:      MetricHistory,
    pub hist_fps:      MetricHistory,
    pub hist_temp_cpu: MetricHistory,
    pub hist_disk:     MetricHistory,
    pub hist_ping:     MetricHistory,

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
    prev_shown_cards: [bool; N_OPTIONAL],
    /// Tracks the compact font size so a window resize fires when the slider moves.
    prev_compact_font_size: f32,
    /// True on the very first frame — fires the initial window size snap.
    startup_resize_pending: bool,
    prev_theme_id: ThemeId,
    /// Frames drawn since the last history tick — logged at debug to watch the repaint rate.
    frames_since_tick: u32,

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
    /// Last tooltip pushed to the tray icon — avoids a Shell_NotifyIcon call per frame.
    #[cfg(windows)]
    last_tray_tip: String,
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let initial_theme_id = CardVisibility::load().theme_id;
        let theme = Theme::from_id(initial_theme_id);
        theme.apply(&cc.egui_ctx);

        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));

        let fps = Arc::new(RwLock::new(FpsSnapshot::default()));
        fps_collector::start(Arc::clone(&fps));

        let ping = Arc::new(RwLock::new(PingSnapshot::default()));

        let vis_init = CardVisibility::load();
        let ping_target_shared = Arc::new(RwLock::new(vis_init.ping_target.clone()));
        ping_collector::start(Arc::clone(&ping), Arc::clone(&ping_target_shared));
        let card_anim_init  = CardAnim::new(&vis_init);
        let prev_shown_init = vis_init.shown();
        log::info!(
            "monitor started: elevated={}, theme {}, {} of {N_OPTIONAL} optional cards shown",
            check_elevated(),
            vis_init.theme_id.label(),
            prev_shown_init.iter().filter(|&&s| s).count()
        );
        let prev_font_size_init = vis_init.compact_font_size;

        // Create card_vis Arc before starting the collector so it can read the selected GPU.
        let card_vis_arc = Arc::new(Mutex::new(vis_init));
        collector::start(Arc::clone(&snapshot), Arc::clone(&card_vis_arc));

        Self {
            snapshot,
            fps,
            ping,
            ping_target_shared,
            theme,
            hist_cpu: MetricHistory::new(HISTORY_LEN),
            hist_mem: MetricHistory::new(HISTORY_LEN),
            hist_rx:       MetricHistory::new(HISTORY_LEN),
            hist_gpu:      MetricHistory::new(HISTORY_LEN),
            hist_fps:      MetricHistory::new(HISTORY_LEN),
            hist_temp_cpu: MetricHistory::new(HISTORY_LEN),
            hist_disk:     MetricHistory::new(HISTORY_LEN),
            hist_ping:     MetricHistory::new(HISTORY_LEN),
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
            prev_theme_id: initial_theme_id,
            frames_since_tick: 0,
            peak_cpu:    0.0,
            peak_mem:    0.0,
            peak_gpu:    0.0,
            peak_disk:   0.0,
            peak_temp:   0.0,
            peak_net_rx: 0.0,
            #[cfg(windows)]
            tray: crate::tray::TrayHandle::build(),
            #[cfg(windows)]
            last_tray_tip: String::new(),
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot, fps_snap: &FpsSnapshot, ping_snap: &PingSnapshot) {
        let is_first = self.first_tick;
        if is_first { self.first_tick = false; }

        // Every history gets exactly one sample per tick so all sparklines share
        // the same 2 s timebase; missing readings push 0 instead of being skipped.
        let n = if is_first { HISTORY_LEN } else { 1 };
        for _ in 0..n {
            self.hist_cpu.push(snap.cpu.total_usage);
            self.hist_mem.push(snap.memory.usage_percent());
            self.hist_rx.push(snap.network.total_rx_bps as f32);
            self.hist_gpu.push(snap.gpu.utilization_percent.unwrap_or(0.0));
            self.hist_fps.push(if fps_snap.active { fps_snap.fps } else { 0.0 });
            self.hist_temp_cpu.push(snap.temps.cpu_celsius.unwrap_or(0.0));
            self.hist_disk.push(snap.disks.first().map(|d| d.usage_percent()).unwrap_or(0.0));
            // Ping timeouts peg the 0–200 ms gauge ceiling so they read as spikes,
            // not as excellent latency.
            let ping_val = if ping_snap.sample_count == 0 {
                0.0
            } else {
                ping_snap.latency_ms.map(|ms| ms as f32).unwrap_or(200.0)
            };
            self.hist_ping.push(ping_val);
        }

        // Update session peaks
        self.peak_cpu    = self.peak_cpu.max(snap.cpu.total_usage);
        self.peak_mem    = self.peak_mem.max(snap.memory.usage_percent());
        self.peak_net_rx = self.peak_net_rx.max(snap.network.total_rx_bps as f32);
        if let Some(u) = snap.gpu.utilization_percent { self.peak_gpu  = self.peak_gpu.max(u); }
        if let Some(d) = snap.disks.first()           { self.peak_disk = self.peak_disk.max(d.usage_percent()); }
        if let Some(t) = snap.temps.cpu_celsius       { self.peak_temp = self.peak_temp.max(t); }
    }

    /// Advance card-collapse animations.
    pub fn advance_card_anims(&mut self, dt: f32, vis: &CardVisibility) {
        const CARD_SPD: f32 = 5.0;  // 200 ms full collapse/expand
        let step = dt * CARD_SPD;
        for (i, &show) in vis.shown().iter().enumerate() {
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
    let n_optional = vis.shown()
        .iter()
        .filter(|&&x| x)
        .count();
    let n = 2 + n_optional;  // CPU + MEM always shown
    // content height: (font_size + padding[8] + bar_reserve[7]).max(min_h[38]); frame overhead: inner_margin(10)+outer(4)=14
    let row_h = (vis.compact_font_size + 15.0).max(38.0) + 14.0;
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
        // Clone card_vis once; use vis_snap for all read-only accesses this frame.
        let vis_snap = self.card_vis.lock().map(|v| v.clone()).unwrap_or_default();

        // Sync ping target into the shared Arc so the ping thread picks up changes.
        if let Ok(mut t) = self.ping_target_shared.write() {
            if *t != vis_snap.ping_target {
                *t = vis_snap.ping_target.clone();
            }
        }

        // Repaint at 60 fps only while card animations are running. Armed passthrough mode
        // polls the Ctrl key every ~100 ms. Otherwise 500 ms is plenty for stats.
        let any_anim = self.card_anim.scale.iter().any(|&s| s > 0.001 && s < 0.999);
        if any_anim {
            ctx.request_repaint();
        } else if vis_snap.passthrough_mode {
            ctx.request_repaint_after(AMBIENT_REPAINT);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Find the main window HWND once (FindWindowW is reliable on Windows)
        if self.hwnd.is_none() {
            self.hwnd = get_main_hwnd();
        }

        // Passthrough and pin-on-top are coupled: one implies the other.
        let passthrough_mode = vis_snap.passthrough_mode;
        if passthrough_mode != self.prev_passthrough_mode {
            if let Ok(mut v) = self.card_vis.lock() {
                v.always_on_top = passthrough_mode;
            }
        }
        self.prev_passthrough_mode = passthrough_mode;

        // Send WindowLevel command when always_on_top changes; also reset opacity state
        // because SetWindowPos (called by winit) can strip WS_EX_LAYERED.
        let always_on_top = vis_snap.always_on_top;
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

        let target_opacity = vis_snap.opacity;
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

        // Re-apply egui style when the user switches theme.
        if vis_snap.theme_id != self.prev_theme_id {
            self.prev_theme_id = vis_snap.theme_id;
            self.theme = Theme::from_id(vis_snap.theme_id);
            self.theme.apply(ctx);
        }

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
        let cur_shown = vis_snap.shown();
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
        let passthrough_armed = vis_snap.passthrough_mode;
        let want_passthrough = passthrough_armed && !ctrl_held();
        if want_passthrough != self.passthrough_active {
            self.passthrough_active = want_passthrough;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(want_passthrough));
        }

        let snap = self.snapshot.read().map(|g| g.clone()).unwrap_or_default();
        let fps_snap = self.fps.read().map(|g| g.clone()).unwrap_or_default();
        let ping_snap = self.ping.read().map(|g| g.clone()).unwrap_or_default();

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
            if tip != self.last_tray_tip {
                tray.set_tooltip(&tip);
                self.last_tray_tip = tip;
            }
            while let Ok(cmd) = tray.rx.try_recv() {
                match cmd {
                    crate::tray::TrayCmd::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    crate::tray::TrayCmd::Exit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

        // Gate history pushes to match the background collector interval
        self.frames_since_tick += 1;
        if self.last_tick.elapsed() >= crate::collector::INTERVAL {
            log::debug!(
                "ui: {:.1} frames/s",
                self.frames_since_tick as f32 / self.last_tick.elapsed().as_secs_f32()
            );
            self.frames_since_tick = 0;
            self.tick_histories(&snap, &fps_snap, &ping_snap);
            self.last_tick = std::time::Instant::now();
        }

        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.advance_card_anims(dt, &vis_snap);

        ui::draw(self, ctx, frame, &snap, &fps_snap, &ping_snap);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop the ETW trace session — it is a kernel object that outlives the process.
        crate::fps_collector::shutdown();
        log::info!("exiting cleanly");
        crate::logging::clear_session_marker();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bscm-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn ambient_repaint_stays_well_above_a_frame() {
        // egui takes ~one 60 Hz frame off the delay; what remains must still be several
        // frames, or the window redraws at vsync rate while a card pulses.
        let frame = std::time::Duration::from_secs_f32(1.0 / 60.0);
        assert!(AMBIENT_REPAINT.saturating_sub(frame) >= frame * 4);
    }

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temp() {
        let dir = scratch_dir("atomic");
        let path = dir.join("config.json");
        write_atomic(&path, "one").unwrap();
        write_atomic(&path, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "temp file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_failure_keeps_the_original() {
        let dir = scratch_dir("atomic-fail");
        let path = dir.join("config.json");
        write_atomic(&path, "good").unwrap();
        // A directory where the target should be: the rename must fail, not delete anything.
        let blocked = dir.join("blocked");
        std::fs::create_dir_all(blocked.join("x")).unwrap();
        assert!(write_atomic(&blocked, "bad").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "good");
        assert!(blocked.join("x").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn describe_changes_names_only_what_changed() {
        let before = r#"{"theme_id":"Jizo","opacity":0.95}"#;
        let after = r#"{"theme_id":"Washi","opacity":0.95,"show_top_cpu":true}"#;
        assert_eq!(
            describe_changes(Some(before), after),
            // serde_json's Map sorts keys (no preserve_order feature).
            r#"show_top_cpu: true, theme_id: "Jizo" -> "Washi""#
        );
        assert_eq!(describe_changes(Some(after), after), "no changes");
        assert_eq!(describe_changes(None, after), "new file");
    }

    #[test]
    fn top_cards_stay_off_for_an_existing_config_but_on_for_a_first_run() {
        let old: CardVisibility = serde_json::from_str(
            r#"{"show_fps":true,"show_gpu":true,"show_net":true,"show_disk":true,"show_temp":true}"#,
        )
        .unwrap();
        assert!(!old.show_top_cpu && !old.show_top_mem);
        let fresh = CardVisibility::default();
        assert!(fresh.show_top_cpu && fresh.show_top_mem);
    }
}

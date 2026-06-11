//! Root application struct — owns shared state and drives the UI each frame.

use std::sync::{Arc, RwLock};

use egui::Context;

use crate::{
    collector, fps_collector,
    models::{FpsSnapshot, MetricHistory, SystemSnapshot, HISTORY_LEN},
    theme::Theme,
    ui,
};

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
    pub always_on_top: bool,
    pub show_about: bool,
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::default();
        theme.apply(&cc.egui_ctx);

        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));
        collector::start(Arc::clone(&snapshot));

        let fps = Arc::new(RwLock::new(FpsSnapshot::default()));
        fps_collector::start(Arc::clone(&fps));

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
            always_on_top: false,
            show_about: false,
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
}

/// Smootherstep: Ken Perlin's C1-continuous S-curve.  Starts slow, eases
/// through the middle, arrives softly — zero first-derivative at both ends.
fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
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

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // 60 fps for smooth animation
        ctx.request_repaint();

        let snap = self.snapshot.read().map(|g| g.clone()).unwrap_or_default();
        let fps_snap = self.fps.read().map(|g| g.clone()).unwrap_or_default();

        // Gate history pushes to match the background collector interval
        if self.last_tick.elapsed() >= crate::collector::INTERVAL {
            self.tick_histories(&snap, &fps_snap);
            self.last_tick = std::time::Instant::now();
        }

        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.advance_displays(dt);

        ui::draw(self, ctx, frame, &snap, &fps_snap);
    }
}

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

    // Lerped display buffers — rendered at 60 fps, smoothly chase hist values
    pub disp_cpu: Vec<f64>,
    pub disp_mem: Vec<f64>,
    pub disp_rx: Vec<f64>,
    pub disp_tx: Vec<f64>,
    pub disp_gpu: Vec<f64>,
    pub disp_fps: Vec<f64>,
    pub disp_temp_cpu: Vec<f64>,
    pub disp_temp_gpu: Vec<f64>,

    last_tick: std::time::Instant,
    first_tick: bool,
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
            disp_cpu: Vec::new(),
            disp_mem: Vec::new(),
            disp_rx: Vec::new(),
            disp_tx: Vec::new(),
            disp_gpu: Vec::new(),
            disp_fps: Vec::new(),
            disp_temp_cpu: Vec::new(),
            disp_temp_gpu: Vec::new(),
            // Subtract 2 s so the first tick fires immediately on first frame
            last_tick: std::time::Instant::now()
                - std::time::Duration::from_secs(2),
            first_tick: true,
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot, fps_snap: &FpsSnapshot) {
        // On startup pre-fill the entire buffer with the current reading so
        // bars appear full immediately instead of growing in over 2 minutes.
        let n = if self.first_tick {
            self.first_tick = false;
            HISTORY_LEN
        } else {
            1
        };

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
        }
    }

    fn advance_displays(&mut self, dt: f32) {
        // Exponential approach: time-constant ≈ 143 ms → visually reaches
        // target in ~0.5 s.  Frame-rate independent via dt.
        let speed = 1.0 - (-dt * 7.0_f32).exp() as f64;
        lerp_buf(&mut self.disp_cpu,      &self.hist_cpu.as_vec(),      speed);
        lerp_buf(&mut self.disp_mem,      &self.hist_mem.as_vec(),      speed);
        lerp_buf(&mut self.disp_rx,       &self.hist_rx.as_vec(),       speed);
        lerp_buf(&mut self.disp_tx,       &self.hist_tx.as_vec(),       speed);
        lerp_buf(&mut self.disp_gpu,      &self.hist_gpu.as_vec(),      speed);
        lerp_buf(&mut self.disp_fps,      &self.hist_fps.as_vec(),      speed);
        lerp_buf(&mut self.disp_temp_cpu, &self.hist_temp_cpu.as_vec(), speed);
        lerp_buf(&mut self.disp_temp_gpu, &self.hist_temp_gpu.as_vec(), speed);
    }
}

fn lerp_buf(disp: &mut Vec<f64>, hist: &[f64], speed: f64) {
    if disp.len() != hist.len() {
        // Size mismatch (startup or history grew) — snap to target immediately
        *disp = hist.to_vec();
        return;
    }
    for (d, &h) in disp.iter_mut().zip(hist.iter()) {
        *d += (h - *d) * speed;
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // 60 fps for smooth animation
        ctx.request_repaint();

        let snap = self.snapshot.read().map(|g| g.clone()).unwrap_or_default();
        let fps_snap = self.fps.read().map(|g| g.clone()).unwrap_or_default();

        // Gate history pushes to 2 s — matching the background collector
        if self.last_tick.elapsed() >= std::time::Duration::from_secs(2) {
            self.tick_histories(&snap, &fps_snap);
            self.last_tick = std::time::Instant::now();
        }

        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.advance_displays(dt);

        ui::draw(self, ctx, frame, &snap, &fps_snap);
    }
}

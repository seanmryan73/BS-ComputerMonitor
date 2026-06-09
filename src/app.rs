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
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot, fps_snap: &FpsSnapshot) {
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

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        let snap = self.snapshot.read().map(|g| g.clone()).unwrap_or_default();
        let fps_snap = self.fps.read().map(|g| g.clone()).unwrap_or_default();

        self.tick_histories(&snap, &fps_snap);

        ui::draw(self, ctx, frame, &snap, &fps_snap);
    }
}

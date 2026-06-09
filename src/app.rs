//! Root application struct — owns shared state and drives the UI each frame.

use std::sync::{Arc, RwLock};

use egui::Context;

use crate::{
    collector,
    models::{MetricHistory, SystemSnapshot, HISTORY_LEN},
    theme::Theme,
    ui,
};

pub struct MonitorApp {
    /// Latest snapshot written by the collector thread.
    snapshot: Arc<RwLock<SystemSnapshot>>,
    pub theme: Theme,

    // ── Rolling histories (kept on the UI side) ───────────────────────────
    pub hist_cpu: MetricHistory,
    pub hist_mem: MetricHistory,
    pub hist_rx: MetricHistory,
    pub hist_tx: MetricHistory,
    pub hist_gpu: MetricHistory,

}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::default();
        theme.apply(&cc.egui_ctx);

        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));
        collector::start(Arc::clone(&snapshot));

        Self {
            snapshot,
            theme,
            hist_cpu: MetricHistory::new(HISTORY_LEN),
            hist_mem: MetricHistory::new(HISTORY_LEN),
            hist_rx: MetricHistory::new(HISTORY_LEN),
            hist_tx: MetricHistory::new(HISTORY_LEN),
            hist_gpu: MetricHistory::new(HISTORY_LEN),
        }
    }

    fn tick_histories(&mut self, snap: &SystemSnapshot) {
        self.hist_cpu.push(snap.cpu.total_usage);
        self.hist_mem.push(snap.memory.usage_percent());
        self.hist_rx.push(snap.network.total_rx_bps as f32);
        self.hist_tx.push(snap.network.total_tx_bps as f32);
        if let Some(u) = snap.gpu.utilization_percent {
            self.hist_gpu.push(u);
        }
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Repaint frequently enough to look alive (collector updates every 2 s).
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Clone the snapshot to avoid holding the read-lock across the UI pass.
        let snap = self
            .snapshot
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.tick_histories(&snap);

        ui::draw(self, ctx, frame, &snap);
    }
}

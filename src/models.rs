//! Core data models shared between the collector and UI layers.

use std::collections::VecDeque;

pub const HISTORY_LEN: usize = 60; // 60 samples × 2 s = 2 minutes of history

// ── Snapshot ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SystemSnapshot {
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub gpu: GpuSnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub network: NetworkSnapshot,
    pub temps: TempSnapshot,
}

// ── CPU ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CpuSnapshot {
    pub total_usage: f32,
    pub per_core: Vec<f32>,
    pub frequency_mhz: u64,
    pub logical_cores: usize,
    pub brand: String,
}

// ── Memory ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
}

impl MemorySnapshot {
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f32 / self.total_bytes as f32 * 100.0
    }
}

// ── GPU ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct GpuSnapshot {
    pub name: String,
    pub utilization_percent: Option<f32>,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
    pub temperature_celsius: Option<f32>,
    pub available: bool,
}

impl GpuSnapshot {
    pub fn vram_usage_percent(&self) -> f32 {
        if self.vram_total_bytes == 0 {
            return 0.0;
        }
        self.vram_used_bytes as f32 / self.vram_total_bytes as f32 * 100.0
    }
}

// ── Disk ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub read_bps: u64,
    pub write_bps: u64,
}

impl DiskSnapshot {
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f32 / self.total_bytes as f32 * 100.0
    }
}

// ── Network ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct NetworkSnapshot {
    pub interfaces: Vec<NetInterface>,
    pub total_rx_bps: u64,
    pub total_tx_bps: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NetInterface {
    pub name: String,
    pub rx_bps: u64,
    pub tx_bps: u64,
}

// ── FPS ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct FpsSnapshot {
    /// Frames per second measured over the last ~1 second window.
    pub fps: f32,
    /// Title of the foreground window being captured.
    pub window_title: String,
    /// True when a capture session is actively running.
    pub active: bool,
}

// ── Temperatures ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct TempSnapshot {
    pub cpu_celsius: Option<f32>,
    pub gpu_celsius: Option<f32>,
}

// ── Rolling history ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MetricHistory {
    pub data: VecDeque<f32>,
    pub max_len: usize,
}

impl MetricHistory {
    pub fn new(max_len: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_len),
            max_len,
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.data.len() >= self.max_len {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn as_vec(&self) -> Vec<f64> {
        self.data.iter().map(|&v| v as f64).collect()
    }

}

// ── Formatting helpers ────────────────────────────────────────────────────────

pub fn fmt_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub fn fmt_bps(bps: u64) -> String {
    const MBPS: u64 = 1_000_000;
    const KBPS: u64 = 1_000;
    if bps >= MBPS {
        format!("{:.1} MB/s", bps as f64 / MBPS as f64)
    } else if bps >= KBPS {
        format!("{:.0} KB/s", bps as f64 / KBPS as f64)
    } else {
        format!("{bps} B/s")
    }
}

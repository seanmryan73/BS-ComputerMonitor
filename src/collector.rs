//! Background data-collection thread.
//!
//! Spawns a single OS thread that refreshes sysinfo every 2 seconds and writes
//! a new [`SystemSnapshot`] into the shared `Arc<RwLock<_>>`.  The UI thread
//! reads from that lock without blocking the collector.

use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System,
};

use crate::models::*;

pub const INTERVAL: Duration = Duration::from_secs(2);

pub fn start(snapshot: Arc<RwLock<SystemSnapshot>>) {
    thread::Builder::new()
        .name("collector".into())
        .spawn(move || run(snapshot))
        .expect("failed to spawn collector thread");
}

fn run(snapshot: Arc<RwLock<SystemSnapshot>>) {
    let refresh = RefreshKind::new()
        .with_cpu(CpuRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything());

    let mut sys = System::new_with_specifics(refresh);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut components = Components::new_with_refreshed_list();

    #[cfg(windows)]
    let mut gpu_col = gpu_win::GpuCollector::new();

    // Let sysinfo warm up before computing first CPU delta.
    sys.refresh_all();
    thread::sleep(Duration::from_millis(700));

    loop {
        let tick = Instant::now();

        sys.refresh_cpu_all();
        sys.refresh_memory();
        disks.refresh();
        networks.refresh();
        components.refresh();

        #[cfg(windows)]
        let (gpu, acpi_cpu_temp) = gpu_win::collect(&mut gpu_col);
        #[cfg(not(windows))]
        let (gpu, acpi_cpu_temp) = (GpuSnapshot::default(), None::<f32>);

        let new_snap = build(&sys, &disks, &networks, &components, gpu, acpi_cpu_temp);

        if let Ok(mut guard) = snapshot.write() {
            *guard = new_snap;
        }

        let elapsed = tick.elapsed();
        if elapsed < INTERVAL {
            thread::sleep(INTERVAL - elapsed);
        }
    }
}

fn build(
    sys: &System,
    disks: &Disks,
    networks: &Networks,
    components: &Components,
    gpu: GpuSnapshot,
    acpi_cpu_temp: Option<f32>,
) -> SystemSnapshot {
    // ── CPU ──────────────────────────────────────────────────────────────────
    let cpus = sys.cpus();
    let cpu = CpuSnapshot {
        total_usage: sys.global_cpu_usage(),
        per_core: cpus.iter().map(|c| c.cpu_usage()).collect(),
        frequency_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
        logical_cores: cpus.len(),
        brand: cpus
            .first()
            .map(|c| c.brand().trim().to_owned())
            .unwrap_or_default(),
    };

    // ── Memory ────────────────────────────────────────────────────────────────
    let memory = MemorySnapshot {
        used_bytes: sys.used_memory(),
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
        swap_used_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
    };

    // ── Disks ─────────────────────────────────────────────────────────────────
    let disk_list = disks
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| DiskSnapshot {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            used_bytes: d.total_space().saturating_sub(d.available_space()),
            total_bytes: d.total_space(),
            read_bps: 0,  // sysinfo 0.32 doesn't expose per-disk I/O rates
            write_bps: 0,
        })
        .collect();

    // ── Network ───────────────────────────────────────────────────────────────
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let interfaces = networks
        .iter()
        .map(|(name, data)| {
            // sysinfo gives delta bytes since last refresh; convert to bytes/sec.
            let secs = INTERVAL.as_secs_f64().max(0.001);
            let rx = (data.received() as f64 / secs) as u64;
            let tx = (data.transmitted() as f64 / secs) as u64;
            total_rx += rx;
            total_tx += tx;
            NetInterface {
                name: name.clone(),
                rx_bps: rx,
                tx_bps: tx,
            }
        })
        .collect();

    let network = NetworkSnapshot {
        interfaces,
        total_rx_bps: total_rx,
        total_tx_bps: total_tx,
    };

    // ── Temperatures ──────────────────────────────────────────────────────────
    // Prefer ACPI thermal zones (no-admin) → fall back to sysinfo components (admin).
    let cpu_temp = acpi_cpu_temp.or_else(|| {
        components
            .iter()
            .find(|c| {
                let lbl = c.label().to_lowercase();
                lbl.contains("package") || lbl.contains("cpu") || lbl.contains("core 0")
            })
            .map(|c| c.temperature())
    });

    let gpu_temp = gpu.temperature_celsius.or_else(|| {
        components
            .iter()
            .find(|c| c.label().to_lowercase().contains("gpu"))
            .map(|c| c.temperature())
    });

    let temps = TempSnapshot {
        cpu_celsius: cpu_temp,
        gpu_celsius: gpu_temp,
    };

    SystemSnapshot {
        cpu,
        memory,
        gpu,
        disks: disk_list,
        network,
        temps,
    }
}

// ── Windows GPU (WMI) ────────────────────────────────────────────────────────

#[cfg(windows)]
mod gpu_win {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};

    use crate::models::GpuSnapshot;

    pub struct GpuCollector {
        com: Option<COMLibrary>,
        conn: Option<WMIConnection>,
        /// Second connection to root\WMI for ACPI thermal zones (no admin needed).
        thermal_conn: Option<WMIConnection>,
        /// Cached GPU name and VRAM from Win32_VideoController (rarely changes)
        cached_name: String,
        cached_vram: u64,
    }

    impl GpuCollector {
        pub fn new() -> Self {
            let mut col = Self {
                com: None,
                conn: None,
                thermal_conn: None,
                cached_name: String::new(),
                cached_vram: 0,
            };
            // Best-effort initialisation; silently degrade if COM unavailable.
            if let Ok(com) = COMLibrary::new() {
                if let Ok(conn) = WMIConnection::new(com.clone()) {
                    col.conn = Some(conn);
                    col.refresh_static();
                }
                // root\WMI holds MSAcpi_ThermalZoneTemperature — readable without admin.
                if let Ok(tc) = WMIConnection::with_namespace_path("ROOT\\WMI", com.clone()) {
                    col.thermal_conn = Some(tc);
                }
                col.com = Some(com);
            }
            col
        }

        fn refresh_static(&mut self) {
            let Some(conn) = &self.conn else { return };

            let result: Result<Vec<HashMap<String, Variant>>, _> = conn.raw_query(
                "SELECT Name, AdapterRAM FROM Win32_VideoController WHERE AdapterRAM > 0",
            );

            let Ok(rows) = result else { return };

            // Prefer discrete GPU: pick the row with the most VRAM.
            let best = rows.iter().max_by_key(|r| {
                match r.get("AdapterRAM") {
                    Some(Variant::UI4(v)) => *v as u64,
                    Some(Variant::UI8(v)) => *v,
                    _ => 0,
                }
            });

            if let Some(row) = best {
                if let Some(Variant::String(name)) = row.get("Name") {
                    self.cached_name = name.clone();
                }
                match row.get("AdapterRAM") {
                    Some(Variant::UI4(v)) => self.cached_vram = *v as u64,
                    Some(Variant::UI8(v)) => self.cached_vram = *v,
                    _ => {}
                }
            }
        }
    }

    /// Returns `(GpuSnapshot, acpi_cpu_temp)`.
    /// `acpi_cpu_temp` is read from `MSAcpi_ThermalZoneTemperature` (no admin required).
    pub fn collect(col: &mut GpuCollector) -> (GpuSnapshot, Option<f32>) {
        let gpu = if col.conn.is_some() {
            let utilization = query_utilization(col);
            let (vram_used, vram_total) = query_vram_dxgi().unwrap_or((0, col.cached_vram));
            GpuSnapshot {
                name: col.cached_name.clone(),
                utilization_percent: utilization,
                vram_used_bytes: vram_used,
                vram_total_bytes: vram_total,
                temperature_celsius: None,
                available: !col.cached_name.is_empty(),
            }
        } else {
            GpuSnapshot::default()
        };
        let cpu_temp = query_acpi_cpu_temp(col);
        (gpu, cpu_temp)
    }

    /// Returns (vram_used_bytes, vram_total_bytes) for the discrete GPU with the most VRAM.
    /// Uses DXGI 1.4 so totals are 64-bit (avoids the 4 GB cap in Win32_VideoController.AdapterRAM).
    fn query_vram_dxgi() -> Option<(u64, u64)> {
        use windows::core::Interface as _;
        use windows::Win32::Graphics::Dxgi::{
            CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory1,
            DXGI_ADAPTER_DESC1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
        };
        const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            let mut best_total: usize = 0;
            let mut best_used: u64 = 0;
            let mut i = 0u32;
            loop {
                let adapter = match factory.EnumAdapters1(i) {
                    Ok(a) => a,
                    Err(_) => break,
                };
                i += 1;
                let mut desc = DXGI_ADAPTER_DESC1::default();
                if adapter.GetDesc1(&mut desc).is_err() {
                    continue;
                }
                if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) != 0 {
                    continue;
                }
                if desc.DedicatedVideoMemory > best_total {
                    best_total = desc.DedicatedVideoMemory;
                    best_used = 0;
                    if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                        let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                        if adapter3
                            .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                            .is_ok()
                        {
                            best_used = info.CurrentUsage;
                        }
                    }
                }
            }
            if best_total > 0 {
                Some((best_used, best_total as u64))
            } else {
                None
            }
        }
    }

    /// Queries MSAcpi_ThermalZoneTemperature in root\WMI.
    /// Returns the maximum sane temperature across all zones — no admin required on most hardware.
    fn query_acpi_cpu_temp(col: &GpuCollector) -> Option<f32> {
        let conn = col.thermal_conn.as_ref()?;
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature")
            .ok()?;

        rows.iter()
            .filter_map(|r| {
                // CurrentTemperature is in tenths of Kelvin (uint32)
                let raw = match r.get("CurrentTemperature") {
                    Some(Variant::UI4(v)) => *v as f64,
                    Some(Variant::I4(v))  => *v as f64,
                    Some(Variant::UI8(v)) => *v as f64,
                    _ => return None,
                };
                let celsius = raw / 10.0 - 273.15;
                // Sanity-check: throw away values outside a physically plausible range
                if celsius > 0.0 && celsius < 150.0 { Some(celsius as f32) } else { None }
            })
            .reduce(f32::max)
    }

    fn variant_as_u64(v: Option<&Variant>) -> Option<u64> {
        match v {
            Some(Variant::UI4(n)) => Some(*n as u64),
            Some(Variant::UI8(n)) => Some(*n),
            Some(Variant::I4(n))  => Some((*n).max(0) as u64),
            Some(Variant::I8(n))  => Some((*n).max(0) as u64),
            _ => None,
        }
    }

    fn query_utilization(col: &GpuCollector) -> Option<f32> {
        let conn = col.conn.as_ref()?;

        // Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine
        // is available on Windows 10 1709+ and groups engine utilisation by
        // adapter and engine type.  We sum the "3D" engines.
        let result: Result<Vec<HashMap<String, Variant>>, _> = conn.raw_query(
            "SELECT Name, UtilizationPercentage \
             FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine",
        );

        let Ok(rows) = result else { return None };
        if rows.is_empty() {
            return None;
        }

        let mut total = 0u64;
        let mut count = 0u32;
        for row in &rows {
            let name_ok = row.get("Name").map_or(false, |v| {
                matches!(v, Variant::String(s) if s.to_lowercase().contains("3d"))
            });
            if !name_ok {
                continue;
            }
            if let Some(v) = variant_as_u64(row.get("UtilizationPercentage")) {
                total += v;
                count += 1;
            }
        }

        if count == 0 {
            // Fallback: average all engines
            for row in &rows {
                if let Some(v) = variant_as_u64(row.get("UtilizationPercentage")) {
                    total += v;
                    count += 1;
                }
            }
        }

        if count > 0 {
            Some((total / count as u64) as f32)
        } else {
            None
        }
    }
}

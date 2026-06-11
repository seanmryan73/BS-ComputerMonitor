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

// ── Windows GPU (PDH + WMI + DXGI) ──────────────────────────────────────────
//
// Utilization and VRAM used come from PDH (Performance Data Helper) because
// WMI perf-formatted classes reset the delta counter on every ExecQuery call,
// always returning 0%.  PDH holds state across calls — matching what Task
// Manager and the C# reference app do.

#[cfg(windows)]
mod gpu_win {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};
    use windows::Win32::System::Performance::{
        PdhAddCounterW, PdhCloseQuery, PdhCollectQueryData,
        PdhGetFormattedCounterArrayW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    };
    use windows::core::PCWSTR;

    use crate::models::GpuSnapshot;

    // PDH handles are plain isize in windows-rs 0.57.
    // GpuCollector lives entirely on the collector thread so Send is safe.
    struct PdhGpuState {
        query:        isize,
        util_counter: isize,
        vram_counter: isize,
    }
    unsafe impl Send for PdhGpuState {}

    impl Drop for PdhGpuState {
        fn drop(&mut self) {
            unsafe { PdhCloseQuery(self.query); }
        }
    }

    pub struct GpuCollector {
        com:          Option<COMLibrary>,
        conn:         Option<WMIConnection>,
        thermal_conn: Option<WMIConnection>,
        cached_name:  String,
        pdh:          Option<PdhGpuState>,
    }

    impl GpuCollector {
        pub fn new() -> Self {
            let mut col = Self {
                com: None, conn: None, thermal_conn: None,
                cached_name: String::new(), pdh: None,
            };
            if let Ok(com) = COMLibrary::new() {
                if let Ok(conn) = WMIConnection::new(com.clone()) {
                    col.conn = Some(conn);
                    col.refresh_gpu_name();
                }
                if let Ok(tc) = WMIConnection::with_namespace_path("ROOT\\WMI", com.clone()) {
                    col.thermal_conn = Some(tc);
                }
                col.com = Some(com);
            }
            col.pdh = init_pdh();
            col
        }

        fn refresh_gpu_name(&mut self) {
            let Some(conn) = &self.conn else { return };
            let Ok(rows): Result<Vec<HashMap<String, Variant>>, _> = conn.raw_query(
                "SELECT Name, AdapterRAM FROM Win32_VideoController WHERE AdapterRAM > 0",
            ) else { return };

            // Prefer the adapter with the most VRAM (= discrete GPU).
            let best = rows.iter().max_by_key(|r| match r.get("AdapterRAM") {
                Some(Variant::UI4(v)) => *v as u64,
                Some(Variant::UI8(v)) => *v,
                _ => 0,
            });
            if let Some(row) = best {
                if let Some(Variant::String(name)) = row.get("Name") {
                    self.cached_name = name.clone();
                }
            }
        }
    }

    /// Initialise PDH query with counters for GPU engine utilisation and
    /// dedicated VRAM usage.  Performs a seed collect so the first real collect
    /// produces a valid rate.
    ///
    /// Counter name detection mirrors the C# reference app: try the WDDM 2.7+
    /// name "Utilization Percentage" first; fall back to the older "% GPU Time".
    fn init_pdh() -> Option<PdhGpuState> {
        use windows::Win32::System::Performance::PdhOpenQueryW;
        unsafe {
            let mut query: isize = 0;
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }

            // Try both known counter names; the C# reference app does the same.
            let util_candidates = [
                "\\GPU Engine(*)\\Utilization Percentage\0",
                "\\GPU Engine(*)\\% GPU Time\0",
            ];
            let mut util_counter: isize = 0;
            let mut util_ok = false;
            for name in &util_candidates {
                let path: Vec<u16> = name.encode_utf16().collect();
                if PdhAddCounterW(query, PCWSTR(path.as_ptr()), 0, &mut util_counter) == 0 {
                    util_ok = true;
                    break;
                }
                util_counter = 0;
            }

            let vram_path: Vec<u16> =
                "\\GPU Adapter Memory(*)\\Dedicated Usage\0".encode_utf16().collect();
            let mut vram_counter: isize = 0;
            let vram_ok = PdhAddCounterW(
                query, PCWSTR(vram_path.as_ptr()), 0, &mut vram_counter,
            ) == 0;

            if !util_ok && !vram_ok {
                PdhCloseQuery(query);
                return None;
            }

            // Seed sample — rate counters return 0 until they have two samples.
            PdhCollectQueryData(query);

            Some(PdhGpuState { query, util_counter, vram_counter })
        }
    }

    pub fn collect(col: &mut GpuCollector) -> (GpuSnapshot, Option<f32>) {
        // Advance PDH — computes delta from the previous seed/collect call.
        if let Some(pdh) = &col.pdh {
            unsafe { PdhCollectQueryData(pdh.query); }
        }

        let utilization = col.pdh.as_ref().and_then(pdh_read_util);
        let vram_used   = col.pdh.as_ref().map_or(0, pdh_read_vram_used);
        let vram_total  = query_vram_total_dxgi().unwrap_or(0);

        let gpu = GpuSnapshot {
            name:                col.cached_name.clone(),
            utilization_percent: utilization,
            vram_used_bytes:     vram_used,
            vram_total_bytes:    vram_total,
            temperature_celsius: None,
            available:           !col.cached_name.is_empty(),
        };
        let cpu_temp = query_acpi_cpu_temp(col);
        (gpu, cpu_temp)
    }

    fn pdh_read_util(pdh: &PdhGpuState) -> Option<f32> {
        if pdh.util_counter == 0 { return None; }
        let values = pdh_counter_doubles(pdh.util_counter)?;
        if values.is_empty() { return None; }

        // Mirror the C# reference app: SUM all engtype_3D instances (not average).
        // GPU Engine instances use the display-adapter LUID, not the physical LUID
        // from DXGI, so filtering by adapter is intentionally skipped here.
        let mut sum_3d   = 0.0f64;
        let mut found_3d = false;
        let mut sum_all  = 0.0f64;
        for (name, val) in &values {
            let v = val.max(0.0);
            sum_all += v;
            if name.to_ascii_lowercase().contains("3d") {
                sum_3d   += v;
                found_3d  = true;
            }
        }
        let raw = if found_3d { sum_3d } else { sum_all };
        Some(raw.min(100.0) as f32)
    }

    fn pdh_read_vram_used(pdh: &PdhGpuState) -> u64 {
        pdh_counter_doubles(pdh.vram_counter)
            .and_then(|v| v.into_iter().map(|(_, b)| b as u64).max())
            .unwrap_or(0)
    }

    /// Returns `(instance_name, value)` pairs from the last PdhCollectQueryData.
    fn pdh_counter_doubles(counter: isize) -> Option<Vec<(String, f64)>> {
        unsafe {
            let mut buf_bytes  = 0u32;
            let mut item_count = 0u32;
            // First call with null buffer returns PDH_MORE_DATA and fills the sizes.
            PdhGetFormattedCounterArrayW(
                counter, PDH_FMT_DOUBLE, &mut buf_bytes, &mut item_count,
                None,
            );
            if item_count == 0 || buf_bytes == 0 { return None; }

            let mut buf = vec![0u8; buf_bytes as usize];
            let status = PdhGetFormattedCounterArrayW(
                counter, PDH_FMT_DOUBLE, &mut buf_bytes, &mut item_count,
                Some(buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );
            if status != 0 { return None; }

            let items = std::slice::from_raw_parts(
                buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                item_count as usize,
            );
            let mut out = Vec::with_capacity(item_count as usize);
            for item in items {
                // Accept CStatus 0 (valid) and 1 (new data); anything higher is an error.
                if item.FmtValue.CStatus > 1 { continue; }
                let name = wide_ptr_to_string(item.szName.0);
                out.push((name, item.FmtValue.Anonymous.doubleValue));
            }
            Some(out)
        }
    }

    unsafe fn wide_ptr_to_string(ptr: *mut u16) -> String {
        if ptr.is_null() { return String::new(); }
        let mut len = 0usize;
        while *ptr.add(len) != 0 { len += 1; }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }

    /// Total dedicated VRAM (bytes) via DXGI — 64-bit, no 4 GB cap.
    fn query_vram_total_dxgi() -> Option<u64> {
        use windows::Win32::Graphics::Dxgi::{
            CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_DESC1,
        };
        const SW_FLAG: u32 = 2; // DXGI_ADAPTER_FLAG_SOFTWARE
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            let mut best: usize = 0;
            let mut i = 0u32;
            loop {
                let adapter = match factory.EnumAdapters1(i) { Ok(a) => a, Err(_) => break };
                i += 1;
                let mut desc = DXGI_ADAPTER_DESC1::default();
                if adapter.GetDesc1(&mut desc).is_err() { continue; }
                if (desc.Flags & SW_FLAG) != 0 { continue; }
                if desc.DedicatedVideoMemory > best { best = desc.DedicatedVideoMemory; }
            }
            if best > 0 { Some(best as u64) } else { None }
        }
    }

    fn query_acpi_cpu_temp(col: &GpuCollector) -> Option<f32> {
        let conn = col.thermal_conn.as_ref()?;
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature")
            .ok()?;
        rows.iter()
            .filter_map(|r| {
                let raw = match r.get("CurrentTemperature") {
                    Some(Variant::UI4(v)) => *v as f64,
                    Some(Variant::I4(v))  => *v as f64,
                    Some(Variant::UI8(v)) => *v as f64,
                    _ => return None,
                };
                let c = raw / 10.0 - 273.15;
                if c > 0.0 && c < 150.0 { Some(c as f32) } else { None }
            })
            .reduce(f32::max)
    }
}

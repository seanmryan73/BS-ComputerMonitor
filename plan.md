# BS Computer Monitor — Build Plan

## Status: v2026.06.14 — compact dashboard complete, overlay + tray working.

---

## Done

- [x] Cargo project scaffold (eframe 0.29, egui_plot, sysinfo 0.32, wmi)
- [x] Background collector thread (CPU, memory, disks, network, temps via sysinfo)
- [x] Windows WMI GPU collector (name, VRAM total via Win32_VideoController)
- [x] DXGI VRAM-used collector (IDXGIAdapter3::QueryVideoMemoryInfo)
- [x] PDH GPU utilization collector (GPU Engine \ Utilization %, "3d" filter, WDDM 2.7+)
- [x] SystemSnapshot data model + fmt helpers
- [x] Dark jewel-tone theme with colour-coded health indicators and CRT scanline overlay
- [x] Frameless window with custom title bar — drag, minimise, resize handles, close
- [x] Compact metric rows — value, sub-label, fill bar, mini sparkline, session-peak tick, pulse animation
- [x] Gradient fill bars (bright left → deep right)
- [x] Catmull-Rom smooth sparklines with gradient fill
- [x] Card glow borders + hover highlight
- [x] JetBrains Mono (values) + Cascadia Mono (labels) embedded fonts
- [x] Per-card height-collapse animation when toggling cards on/off
- [x] Rolling history buffers (60 samples × 2 s = 2 min)
- [x] FPS collector — foreground app/game frame rate (WGC fallback from ETW)
- [x] Settings window — separate opaque OS window, positions left of main window
- [x] Card visibility toggles (FPS, GPU, NET, DISK, TEMP) — persisted to config
- [x] Window opacity slider (Win32 SetLayeredWindowAttributes) — persisted
- [x] Value font-size slider (11–60 pt) — window height auto-snaps
- [x] Network bandwidth cap selector (10 Mbps–10 Gbps) — persisted
- [x] GPU adapter selector — persisted
- [x] JSON config persistence (`%APPDATA%\BSComputerMonitor\config.json`)
- [x] Game overlay / passthrough mode — MousePassthrough + hold-Ctrl to interact
- [x] Crosshair button in top-left of title bar toggles passthrough on/off
- [x] Passthrough auto-couples pin-on-top (session-only, reset to OFF on restart)
- [x] Reset to defaults button in settings
- [x] System tray — minimise to tray, live CPU/GPU tooltip, Show/Exit menu
- [x] `.gitignore`, `README.md`, `plan.md`
- [x] Dead-code cleanup — removed interpolation layer (prev_*, disp_*, smootherstep, interp_buf), hist_tx, hist_temp_gpu

---

## Next — Missing Core Metrics

- [ ] **Disk I/O speeds** (read/write bytes/sec) — sysinfo 0.32 doesn't expose this; implement via Windows `DeviceIoControl` + `DISK_PERFORMANCE` or `pdh` crate
- [ ] **CPU temperature fallback** — if sysinfo component gives nothing, try OHM WMI provider or WinRing0

---

## Next — New Features

- [ ] **Process tab** — top-N processes sorted by CPU/memory using `sysinfo::Processes`
- [ ] **Theme switcher** — `ThemeKind` enum in `MonitorApp`, toolbar button cycling through presets (Dark, Miami, Arctic, Neon)
- [ ] **History tab** — full-size chart for CPU + memory over 10 min / 24 h; persist rollups to `%APPDATA%\BSComputerMonitor\history.json`
- [ ] **Alert notifications** — Windows toast when CPU/memory/temp crosses threshold

---

## Known Limitations

| Issue | Root cause | Fix path |
|-------|-----------|----------|
| No disk I/O speeds | sysinfo 0.32 limitation | Windows DeviceIoControl or pdh crate |
| GPU utilization may show N/A | Requires Win10 1709+ WMI perf counters + admin | Fallback to DXGI or nvml-wrapper |
| Temps may be empty | Needs admin or compatible sensors | LHM/OHM WMI provider fallback |
| No per-process network | ETW complexity | etw-reader crate |
| RTX 5040 (Blackwell) GPU not working | See investigation note below | Needs machine access to diagnose |

---

## RTX 5040 (Blackwell) GPU — Pending Investigation

**Requires access to the RTX 5040 machine to diagnose and verify.**

### Background
The app uses standard Windows APIs (WMI, PDH, DXGI) — no NVML — so this is not a driver
SDK version issue.  The most likely culprit is the PDH 3D engine name filter in
`src/collector.rs` `pdh_read_util()`.

### How GPU utilization is collected
`pdh_read_util()` opens the counter `\\GPU Engine(*)\\Utilization Percentage` (WDDM 2.7+)
or `\\GPU Engine(*)\\% GPU Time` (older), then sums only the instances whose name contains
`"3d"` to match Task Manager's "3D" row.

Blackwell (RTX 50 series) changed how WDDM reports engine types in PDH instance names.
If the RTX 5040's compute/3D engines no longer carry the string `"3d"`, utilization
reads as 0 and the card shows as unavailable.

### Step 1 — Run a diagnostic build on the 5040 machine
Add a temporary `eprintln!` (or write to a log file) inside `pdh_read_util()` that dumps
every instance name returned by `PdhGetFormattedCounterArrayW` before the `"3d"` filter.
This tells us exactly what strings Blackwell uses.

The relevant code is in `src/collector.rs` around the `pdh_read_util` function:
```rust
// Temporary diagnostic — print all PDH GPU engine instance names
for item in &items {
    eprintln!("[GPU-DIAG] instance: {}", item.szName.to_string().unwrap_or_default());
}
```

Run with `cargo run 2> gpu_diag.txt` and share `gpu_diag.txt`.

### Step 2 — Fix the filter
Depending on what the diagnostic shows, one of:

**Option A — Update the string filter** if Blackwell uses a different but consistent name
(e.g. `"compute"` or `"3D"` with different capitalisation):
```rust
// In pdh_read_util(), change:
if name.to_ascii_lowercase().contains("3d") { ... }
// to whatever Blackwell actually uses
```

**Option B — Broaden the fallback** if no matching 3D instances are found, sum all
engines except known non-compute ones (`"videodecode"`, `"videoprocessing"`, `"copy"`):
```rust
// If no "3d" instances found, fall back to summing everything except decode/copy
let filtered: Vec<_> = items.iter().filter(|i| {
    let n = i.szName.to_ascii_lowercase();
    !n.contains("videodecode") && !n.contains("videoprocessing") && !n.contains("copy")
}).collect();
```

**Option C — Match Task Manager exactly** by reading the engine type field from the
instance name (WDDM encodes it as `luid_0x…_phys_0_eng_0_engtype_3D`) — parse the
`engtype_` suffix instead of matching on `"3d"`.

### Step 3 — Verify
On the 5040 machine, confirm the GPU card shows utilization, VRAM used/total, and
temperature.  Also check that WMI `Win32_VideoController` picks up the RTX 5040 name
correctly (the `WHERE AdapterRAM > 0` filter and max-VRAM selection should be fine, but
worth verifying if the name field is blank).

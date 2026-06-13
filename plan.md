# BS Computer Monitor — Build Plan

## Status: v1.96.9 — core dashboard + overlay features complete and running.

---

## Done

- [x] Cargo project scaffold (eframe 0.29, egui_plot, sysinfo 0.32, wmi)
- [x] Background collector thread (CPU, memory, disks, network, temps via sysinfo)
- [x] Windows WMI GPU collector (name, VRAM, utilization %)
- [x] SystemSnapshot data model + fmt helpers
- [x] Dark cyberpunk theme with colour-coded health indicators and CRT scanline overlay
- [x] Frameless window with custom title bar — drag, minimise, resize handles, close
- [x] 7 metric cards in a responsive 2-column grid (CPU, MEM, FPS, GPU, NET, DISK, TEMP)
- [x] Sparklines via egui_plot
- [x] Rolling history buffers (60 samples × 2 s = 2 min)
- [x] Smooth animated metric transitions (smootherstep interpolation, 60 fps)
- [x] FPS collector — foreground app/game frame rate
- [x] Help / About / Config window — separate opaque OS window via egui multi-viewport, positions to the left of the main window
- [x] Card visibility toggles (FPS, GPU, NET, DISK, TEMP) — persisted to config
- [x] Window opacity slider (Win32 `SetLayeredWindowAttributes`) — persisted
- [x] Compact display mode — numbers only, no graphs, narrower window, adjustable font size
- [x] JSON config persistence (`%APPDATA%\BSComputerMonitor\config.json`)
- [x] Game overlay / passthrough mode — `ViewportCommand::MousePassthrough` + hold-Ctrl to interact (`GetAsyncKeyState`)
- [x] Crosshair button in top-left of title bar toggles passthrough on/off
- [x] Passthrough auto-couples pin-on-top (bidirectional — both session-only, reset to OFF on restart)
- [x] Right-panel card numbers use compact-style glow (4-cardinal halo, alpha 18)
- [x] Reset to defaults button in Help/About
- [x] Help/About contains passthrough instructions with inline crosshair icon (no duplicate toggle)
- [x] `.gitignore`, `README.md`, `plan.md`

---

## Next — Visual Polish

- [ ] **Gradient bars** — paint fill with a left→right colour fade using `egui::Painter::mesh()`; cyan→teal for CPU, amber→orange for memory
- [ ] **Glow on card borders** — subtle outer shadow via multiple strokes at decreasing alpha
- [ ] **Custom font** — embed JetBrains Mono via `egui::FontDefinitions`; add it as a file in `assets/`
- [ ] **Card hover highlight** — faint border brighten on hover; tune the colour

---

## Next — Missing Core Metrics

- [ ] **Disk I/O speeds** (read/write bytes/sec) — sysinfo 0.32 doesn't expose this; implement via Windows `DeviceIoControl` + `DISK_PERFORMANCE` or `pdh` crate
- [ ] **GPU VRAM used** — WMI `Win32_VideoController` only gives total; use DXGI `IDXGIAdapter3::QueryVideoMemoryInfo` (feature `Win32_Graphics_Dxgi`) for real-time usage
- [ ] **CPU temperature fallback** — if sysinfo component gives nothing, try OHM WMI provider or a direct WinRing0 approach

---

## Next — New Features

- [ ] **Process tab** — top-N processes sorted by CPU/memory using `sysinfo::Processes`; add a bottom panel
- [ ] **Theme switcher** — store `ThemeKind` in `MonitorApp`, toolbar button cycling through presets (Dark, Miami, Arctic, Neon)
- [ ] **History tab** — full-size chart for CPU + memory over 10 min / 24 h / 7 days; persist rollups to `%APPDATA%\BSComputerMonitor\history.json`
- [ ] **Watchlist** — pin processes from process tab; per-pid sparkline
- [ ] **GPU adapter selection** — dropdown when multiple adapters detected; persist selection to config
- [ ] **System tray** — minimize to tray; `tray-icon` crate
- [ ] **Alert notifications** — Windows toast when CPU/memory/temp crosses threshold

---

## Known Limitations

| Issue | Root cause | Fix path |
|-------|-----------|----------|
| No disk I/O speeds | sysinfo 0.32 limitation | Windows DeviceIoControl or pdh crate |
| GPU utilization may show N/A | Requires Win10 1709+ WMI perf counters | Fallback to DXGI or nvml-wrapper |
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

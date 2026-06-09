# BS Computer Monitor — Build Plan

## Status: v1 core dashboard complete, compiles & builds.

---

## Done

- [x] Cargo project scaffold (eframe 0.29, egui_plot, sysinfo 0.32, wmi)
- [x] Background collector thread (CPU, memory, disks, network, temps via sysinfo)
- [x] Windows WMI GPU collector (name, VRAM, utilization %)
- [x] SystemSnapshot data model + fmt helpers
- [x] Dark cyberpunk theme (matches C# original palette)
- [x] Frameless window with custom title bar (drag / min / close)
- [x] 6 metric cards in a responsive 2-column grid
- [x] Sparklines via egui_plot
- [x] Rolling history buffers (60 samples × 2 s = 2 min)
- [x] `.gitignore`, `README.md`, `plan.md`

---

## Next — Visual Polish (do first, biggest impact)

- [ ] **Gradient bars** — paint fill with a left→right colour fade using `egui::Painter::mesh()`; the C# version uses cyan→teal gradients for CPU, amber→orange for memory
- [ ] **Glow effect on card borders** — subtle outer shadow using multiple strokes at decreasing alpha
- [ ] **Animated value transitions** — lerp displayed value toward real value over ~300 ms so numbers don't jump
- [ ] **Custom font** — embed JetBrains Mono via `egui::FontDefinitions`; add it as a file in `assets/`
- [ ] **Card hover highlight** — faint border brighten on hover already wired into egui visuals; tune the colour

---

## Next — Missing Core Metrics

- [ ] **Disk I/O speeds** (read/write bytes/sec) — sysinfo 0.32 doesn't expose this; implement via Windows `DeviceIoControl` + `DISK_PERFORMANCE` or `NtQuerySystemInformation` SystemDiskInformationClass
- [ ] **GPU VRAM used** — WMI `Win32_VideoController` only gives total; use DXGI `IDXGIAdapter3::QueryVideoMemoryInfo` (add `windows` crate feature `Win32_Graphics_Dxgi`) to get real-time usage
- [ ] **CPU temperature fallback** — if sysinfo component gives nothing, try `OpenHardwareMonitorLib` WMI provider (requires OHM running) or a direct WinRing0 approach

---

## Next — New Features (from C# original)

- [ ] **Process tab** — top-N processes sorted by CPU/memory, using `sysinfo::Processes`; add a `TabWindow`-style bottom panel
- [ ] **Theme switcher** — store `ThemeKind` in `MonitorApp`, add toolbar button cycling through 3–5 presets (Dark, Miami, Arctic, Neon)
- [ ] **History tab** — full-size chart for CPU + memory over 10 min / 24 h / 7 days; extend `HistoryStoreService` to persist rollups to `%APPDATA%\BSComputerMonitor\history.json`
- [ ] **In-game overlay** — always-on-top transparent secondary `eframe` window; requires spawning a second `EventLoop` or using Tauri/winit directly; egui's multi-viewport API (added in 0.28) may be the cleanest path
- [ ] **Watchlist** — pin processes from the process tab; separate aggregation + sparkline per watched pid
- [ ] **GPU adapter selection** — dropdown when multiple adapters detected; persist selection to `%APPDATA%\BSComputerMonitor\gpu-selection.json`

---

## Nice-to-have

- [ ] System tray icon (minimize to tray) — `tray-icon` crate
- [ ] Settings panel — sampling interval, alert thresholds, module toggles
- [ ] Alert notifications — Windows toast when CPU/memory/temp crosses threshold (`windows-notifications` or `notify-rust`)
- [ ] Release build + NSIS/WiX installer

---

## Known Limitations

| Issue | Root cause | Fix path |
|-------|-----------|----------|
| No disk I/O speeds | sysinfo 0.32 limitation | Windows DeviceIoControl or pdh crate |
| GPU utilization may show N/A | Requires Win10 1709+ WMI perf counters | Fallback to DXGI or nvml-wrapper |
| Temps may be empty | Needs admin or compatible sensors | LHM/OHM WMI provider fallback |
| No per-process network | Skipped (ETW) | Add later with etw-reader crate |

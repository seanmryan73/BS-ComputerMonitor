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

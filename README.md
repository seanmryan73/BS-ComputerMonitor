# BS Computer Monitor

A blazing-fast Windows system monitor built in Rust with an egui GPU-accelerated UI.  
Ported from the C# WPF original at `C:\_repos\ComputerMonitor`.

## Features (v1 — core dashboard)

| Card | Metrics |
|------|---------|
| CPU | Total usage %, per-core mini-bars, sparkline, frequency, brand |
| Memory | Usage %, used/available/swap, sparkline |
| GPU | Utilization % (WMI), VRAM total, temperature (best-effort) |
| Network | TX/RX speeds, dual sparklines, per-interface breakdown |
| Disk | Per-volume fill bars with used/total |
| Temps | CPU & GPU °C — admin not required but improves sensor access |

- Frameless custom-chrome window (drag, minimise, close)
- Dark cyberpunk colour palette with colour-coded health indicators
- 2-column responsive grid layout
- Background collector thread — zero UI-thread blocking
- ~5 MB binary, minimal CPU/memory overhead at runtime

## Requirements

- Windows 10 1709+ (for GPU WMI perf counters)
- Rust toolchain (stable, MSVC target)

## Build & Run

```powershell
cargo run                   # debug
cargo run --release         # optimised (~5 MB binary)
```

## Architecture

```
collector thread (2 s interval)
  sysinfo  ──► CPU, memory, disks, network, component temps
  WMI      ──► GPU name, VRAM, utilization (Win32_VideoController
               + GPUPerformanceCounters_GPUEngine)
      │
      ▼
Arc<RwLock<SystemSnapshot>>
      │
UI thread (egui, ~60 fps)
  MonitorApp::update() ── reads snapshot, ticks histories, draws cards
```

## Roadmap

See [plan.md](plan.md) for the detailed next-steps backlog.

# BS Computer Monitor

A blazing-fast Windows system monitor built in Rust with an egui GPU-accelerated UI.  
Ported from the C# WPF original at `C:\_repos\ComputerMonitor`.

**Version 1.96.9**

## Features

| Card | Metrics |
|------|---------|
| CPU | Total usage %, per-core mini-bars, sparkline, frequency, brand |
| Memory | Usage %, used/available/swap, sparkline |
| FPS | Foreground game/app frame rate (toggleable) |
| GPU | Utilization %, temperature, VRAM total (WMI) |
| Network | TX/RX speeds, dual sparklines, per-interface breakdown |
| Disk | Per-volume fill bars with used/total |
| Temps | CPU & GPU °C — admin not required but improves sensor access |

### Window & UI

- Frameless custom-chrome window with drag, minimise, resize handles, and close
- Dark cyberpunk colour palette with colour-coded health indicators
- CRT scanline overlay effect
- Smooth animated metric transitions (smootherstep interpolation, 60 fps)
- 2-column responsive grid layout

### Display modes

- **Normal mode** — full cards with sparklines/bar graphs
- **Compact mode** — numbers only, no graphs, narrower window; adjustable value font size

### Configuration (persisted to `%APPDATA%\BSComputerMonitor\config.json`)

- Toggle individual cards on/off (FPS, GPU, NET, DISK, TEMP)
- Window opacity — 15% to 100% via Win32 `SetLayeredWindowAttributes`
- Compact mode toggle + font size slider
- Reset to defaults

### Game overlay / passthrough

- Click-through passthrough mode — the monitor floats over your game, all clicks pass through to the app behind
- Toggled via the **crosshair button** in the top-left of the title bar
- Passthrough auto-enables pin-on-top so the window stays above your game
- Hold **Ctrl** to temporarily capture input while passthrough is active (drag, resize, toggle off)
- Both passthrough and pin-on-top always reset to OFF on restart — close the app any time without getting stuck

### Help / About / Config

- Click **?** in the title bar to open the settings panel as a separate opaque OS window
- Positions to the left of the main window so you can see changes live
- Contains all configuration controls plus passthrough usage instructions

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
  MonitorApp::update()
    ├─ apply Win32 opacity (SetLayeredWindowAttributes)
    ├─ passthrough logic (GetAsyncKeyState / ViewportCommand::MousePassthrough)
    ├─ pin-on-top auto-coupled to passthrough (ViewportCommand::WindowLevel)
    ├─ smootherstep interpolation between collector ticks
    └─ draw: titlebar · cards (normal or compact) · about viewport · scanlines

Config
  Arc<Mutex<CardVisibility>>  ── shared between main viewport and about viewport
  serde_json ──► %APPDATA%\BSComputerMonitor\config.json
  passthrough + pin-on-top are session-only (#[serde(skip)], always OFF on launch)
```

## Roadmap

See [plan.md](plan.md) for the detailed next-steps backlog.

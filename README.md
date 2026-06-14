# BS Computer Monitor

A blazing-fast Windows system monitor built in Rust with an egui GPU-accelerated UI.  
Ported from the C# WPF original at `C:\_repos\ComputerMonitor`.

**Version 2026.06.14**

## Features

| Card | Metrics |
|------|---------|
| CPU | Usage %, frequency (GHz), core count, sparkline, session peak |
| Memory | Usage %, used / total, sparkline, session peak |
| FPS | Foreground game/app frame rate, window title (toggleable) |
| GPU | Utilization %, VRAM used / total, sparkline, session peak (toggleable) |
| Network | RX / TX speeds, bandwidth fill bar vs configurable cap, sparkline (toggleable) |
| Disk | First-volume usage %, free space, sparkline, session peak (toggleable) |
| Temp | CPU & GPU °C, sparkline, session peak (toggleable) |

Each card shows a gradient fill bar, a ghost sparkline trend, and a white session-peak tick.  
Warn/crit thresholds trigger animated pulsing glow rings.

### Window & UI

- Frameless custom-chrome window — drag, minimise, resize handles, close
- Dark jewel-tone palette (sapphire CPU · amber MEM · amethyst GPU · jade NET · lapis DISK · fire-opal TEMP)
- Colour-coded health indicators: emerald → amber gold → ruby
- CRT scanline overlay
- Hover highlight on every card
- Per-card height-collapse animation when cards are toggled on/off

### Display

Compact mode — value + sub-label + fill bar + sparkline per row.  
Font size is freely adjustable (11 – 60 pt) via the settings panel; the window height auto-snaps to fit.

### Configuration (persisted to `%APPDATA%\BSComputerMonitor\config.json`)

- Toggle individual cards on/off — FPS, GPU, NET, DISK, TEMP
- Value font size slider
- Window opacity — 15 % to 100 % via Win32 `SetLayeredWindowAttributes`
- Network bandwidth cap (10 Mbps → 10 Gbps) — sets 100 % on the NET fill bar
- GPU adapter selector (multi-GPU systems)
- Reset to defaults

### Game overlay / passthrough

- Click-through passthrough mode — the monitor floats over your game, all clicks pass through
- Toggled via the **crosshair button** in the top-left of the title bar
- Passthrough auto-enables pin-on-top
- Hold **Ctrl** while passthrough is active to temporarily capture input (drag, resize, toggle off)
- Both passthrough and pin-on-top always reset to OFF on restart

### Help / Settings

- Click **?** in the title bar to open the settings panel as a separate opaque OS window
- Positions to the left of the main window so changes are visible live

### System tray

- Minimise-to-tray support with live CPU / GPU tooltip
- Right-click menu: **Show** / **Exit**

## Requirements

- Windows 10 1709+ (WDDM 2.7+ for GPU PDH perf counters)
- Rust toolchain — stable, MSVC target (`x86_64-pc-windows-msvc`)
- Admin rights recommended (GPU utilization % and CPU/GPU temps need elevated access)

## Build & Run

```powershell
cargo run                   # debug
cargo run --release         # optimised (~5 MB binary)
```

## Architecture

```
collector thread (2 s interval)
  sysinfo  ──► CPU, memory, disks, network, component temps
  WMI      ──► GPU name, VRAM total (Win32_VideoController)
  PDH      ──► GPU utilization % (GPU Engine \ Utilization Percentage, "3d" filter)
  DXGI     ──► VRAM used (IDXGIAdapter3::QueryVideoMemoryInfo)
      │
      ▼
Arc<RwLock<SystemSnapshot>>        Arc<RwLock<FpsSnapshot>>  ◄── fps_collector thread
      │                                        │                    (WGC / ETW)
      └─────────────────┬─────────────────────┘
                        ▼
UI thread (egui, 60 fps)
  MonitorApp::update()
    ├─ Win32 opacity   (SetLayeredWindowAttributes)
    ├─ passthrough     (GetAsyncKeyState + ViewportCommand::MousePassthrough)
    ├─ pin-on-top      (ViewportCommand::WindowLevel, auto-coupled to passthrough)
    ├─ card animations (height-collapse squish per optional card)
    ├─ system tray     (tooltip update + Show/Exit commands)
    └─ draw: titlebar · compact cards · settings viewport · scanlines

Config
  Arc<Mutex<CardVisibility>>  ── shared between main viewport and settings viewport
  serde_json ──► %APPDATA%\BSComputerMonitor\config.json
  passthrough + pin-on-top are session-only (#[serde(skip)], always OFF on launch)
```

## Known limitations

| Issue | Root cause | Fix path |
|-------|-----------|----------|
| No disk I/O speeds | sysinfo 0.32 limitation | Windows `DeviceIoControl` + `DISK_PERFORMANCE` or `pdh` crate |
| GPU util may show N/A | Needs WDDM 2.7+ and admin | DXGI / nvml-wrapper fallback |
| Temps may be empty | Needs admin or compatible sensors | LHM/OHM WMI provider fallback |
| RTX 50-series (Blackwell) GPU util | PDH engine name may differ from `"3d"` | Diagnostic needed — see plan.md |

## Roadmap

See [plan.md](plan.md) for the detailed backlog.

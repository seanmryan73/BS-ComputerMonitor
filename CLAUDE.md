# BS-ComputerMonitor — Claude Context

## What this repo is

egui/eframe Windows desktop system monitor. Displays CPU, GPU, memory, disk, and network stats as live cards with per-metric accent colours, a ping latency panel, an FPS overlay, and a system tray icon. Metrics from sysinfo (general stats), WMI (hardware temps), ETW (FPS counting), and WGC/DX11 (GPU capture).

## Reference notes (read these for standards)

- Rust desktop standards: `C:\_repos\Obsidian\Notes\Claude\Reference\Rust-Desktop-Standards.md`
- Author / version / company: `C:\_repos\Obsidian\Notes\Claude\Reference\Author-Version-Standards.md`
- Project details: `C:\_repos\Obsidian\Notes\Claude\Projects\BS-ComputerMonitor Claude Context.md`

## Author / version standard

- Author: Sean Ryan <seanmryan@gmail.com>
- Company: BagPipes
- Version format: `YYYY.MM.DD`

## Pinned dependency versions — do not change without a concrete reason

| Crate | Pinned version | Why |
|-------|---------------|-----|
| egui / eframe | `0.29` | Intentionally pinned; upgrade all egui-family crates together or not at all |
| sysinfo | `0.32` | 0.33 has breaking API changes: `RefreshKind::new()` removed, `refresh(bool)` signature change, `temperature()` returns `Option<Option<f32>>` |
| windows | `0.57` | Must stay aligned with `wmi = 0.13` transitive dependency |

## Key constraints

- **`unsafe` is required** — 25+ unsafe blocks for ETW, DX11, and WGC Windows API calls. Do NOT add `#![deny(unsafe_code)]`.
- **Theme:** Custom per-metric `Theme` struct, not the standard 4-theme `ThemeManager`. Coral Storm palette applied to it.
- **Console in debug:** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — the terminal is visible in debug for `env_logger`. Intentional; do not change.
- **`windows` + `wmi` must be updated together** — transitive version coupling.
- **Minimal edits** — prefer targeted changes; do not refactor surrounding code unless the task requires it.

# BS-ComputerMonitor — Claude Context

## What this repo is

egui/eframe Windows desktop system monitor. Displays CPU, GPU, memory, disk, and network stats as live cards with per-metric accent colours, a ping latency panel, an FPS overlay, and a system tray icon. Metrics from sysinfo (general stats), WMI (hardware temps), ETW (FPS counting), and WGC/DX11 (GPU capture).

## Shared reference notes

@c:\_repos\Obsidian\Notes\Claude\Reference\Author-Version-Standards.md
@c:\_repos\Obsidian\Notes\Claude\Reference\Rust-Desktop-Standards.md

## Project context

@c:\_repos\Obsidian\Notes\Claude\Projects\BS-ComputerMonitor Claude Context.md

## Pinned dependency versions — do not change without a concrete reason

| Crate | Pinned version | Why |
|-------|---------------|-----|
| egui / eframe | `0.29` | Intentionally pinned; upgrade all egui-family crates together or not at all |
| sysinfo | `0.32` | 0.33 has breaking API changes: `RefreshKind::new()` removed, `refresh(bool)` signature change, `temperature()` returns `Option<Option<f32>>` |
| windows | `0.57` | Must stay aligned with `wmi = 0.13` transitive dependency |

## Key constraints

- **`unsafe` is required** — 25+ unsafe blocks for ETW, DX11, and WGC Windows API calls. Do NOT add `#![deny(unsafe_code)]`.
- **Theme:** Custom per-metric `Theme` struct with a `ThemeId` enum (5 themes: CoralStorm, CandyPop, GlitchMode, ColdSteel, Lucky — matching `Rust-Desktop-Standards.md`). `theme_id` is persisted in `CardVisibility`/`config.json`. Live switching via `prev_theme_id` diff in `update()`. Metric accents (cpu/mem/gpu/net/disk/temp) are shared constants across all themes — only surfaces and text colours change per theme.
- **Console in debug:** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — the terminal is visible in debug for `env_logger`. Intentional; do not change.
- **`windows` + `wmi` must be updated together** — transitive version coupling.
- **Minimal edits** — prefer targeted changes; do not refactor surrounding code unless the task requires it.

## After this session

When the session ends or the user says to wrap up, update the project context note:
`c:\_repos\Obsidian\Notes\Claude\Projects\BS-ComputerMonitor Claude Context.md`

Update these sections:
- **Current constraints** — add any new version pins, banned patterns, or architecture rules discovered
- **Fix history** — add bugs fixed with root cause (one line each: date · symptom · cause · fix)
- **Next actions** — replace with the current list
- **frontmatter `version:`** — set to today's date (YYYY.MM.DD)

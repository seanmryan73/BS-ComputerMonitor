# BS-ComputerMonitor — Agent Context

## What this repo is

egui/eframe Windows desktop system monitor. Displays CPU, GPU, memory, disk, and network stats as live cards with per-metric accent colours, a ping latency panel, an FPS overlay, and a system tray icon. Metrics from sysinfo (general stats), WMI (hardware temps), ETW (FPS counting), and WGC/DX11 (GPU capture).

> **Agent context.** Lines beginning with `@` are absolute file paths (one per line; paths may
> contain spaces). Claude Code loads them automatically. All other agents (Codex, Copilot, etc.): read
> every `@` file now, before doing any work. The *Read before you…* table is **not** loaded up front:
> open the named note when your task matches the row, before you change anything in that area.

## Always loaded

@c:\_repos\Obsidian\Notes\Claude\Reference\Author-Version-Standards.md
@c:\_repos\Obsidian\Notes\Claude\Reference\Core-Rules.md
@c:\_repos\Obsidian\Notes\Claude\Reference\Core-Rules-Rust.md
@c:\_repos\Obsidian\Notes\Claude\Projects\BS-ComputerMonitor Claude Context.md

## Read before you…

| Before you… | Read (path under `c:\_repos\Obsidian\Notes\Claude\`) |
|---|---|
| change UI, layout, themes, settings persistence or app architecture | `Reference/Rust-Desktop-Standards.md` (the matching section) |
| call a Windows API, touch audio, capture or displays, or spawn a process | `Reference/Windows-Platform-Gotchas.md` (the matching section) |
| recommend an architecture, a dependency or a pattern | search `Reference/Cross-Project-Anti-Patterns.md` |

Until 2026-09-19 these standards were `@`-loaded in full at every session start. See
`Reference/Multi-Tool-AI-Setup.md` rule 6 for the two-tier loading and its budget.

## Pinned dependency versions — do not change without a concrete reason

| Crate | Pinned version | Why |
|-------|---------------|-----|
| egui / eframe | `0.29` | Intentionally pinned; upgrade all egui-family crates together or not at all |
| sysinfo | `0.32` | 0.33 has breaking API changes: `RefreshKind::new()` removed, `refresh(bool)` signature change, `temperature()` returns `Option<Option<f32>>` |
| windows | `0.57` | Must stay aligned with `wmi = 0.13` transitive dependency |

## Key constraints

- **`unsafe` is required** — 25+ unsafe blocks for ETW, DX11, and WGC Windows API calls. Do NOT add `#![deny(unsafe_code)]`.
- **Theme:** Custom per-metric `Theme` struct with a `ThemeId` enum (8 themes: CoralStorm, Shibui, Kasane, ColdSteel, Jizo, HotSteel, Terminal, Washi — matching `Theme-Palettes-and-Templates.md` 2026.10.10h; this app maps the vault tokens onto its own surface fields rather than using the template's `AppTheme`). Jizo and Washi are **light**: `apply()` picks `Visuals::light()` from `is_light()` (background luminance), so never hard-code white or a dark-only colour outside `theme.rs`. Default is `ColdSteel`, per the standard. `theme_id` is persisted in `CardVisibility`/`config.json`. Live switching via `prev_theme_id` diff in `update()`. Metric accents (cpu/mem/gpu/net/disk/temp) and ok/warn/crit, which also drive the hover/active glow via `accent_cpu`, are shared constants across the dark themes; Coral Storm, Jizo and Washi carry the same hues at a lightness that reaches 4.5:1 on their surfaces. `every_theme_meets_contrast_minimums` in `theme.rs` gates all of it.
- **Console in debug:** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — the terminal is visible in debug for `env_logger`. Intentional; do not change.
- **Elevation:** release builds embed a `requireAdministrator` manifest from `build.rs`, gated on `PROFILE == "release"`. Debug and `cargo test` builds run unelevated on purpose: the test harness would otherwise fail with os error 740, and debug keeps the About screen's "not elevated" state in use. Do not make the manifest unconditional. Auto-start at login would need a Task Scheduler task with highest privileges, because Windows skips elevated exes in the Run key and the Startup folder.
- **Settings/Help/About window (`src/ui/about.rs`):** three tabs of collapsible sections, with Reset/Close pinned outside the scroll area. Its colours come only from the active `Theme`, and its secondary text is blended from `text_primary`, not `text_subtle`. `secondary_text_stays_readable_on_every_theme` holds that text to 4.5:1, so a new theme must pass it. Reset asks for confirmation first.
- **rustfmt is not repo-wide:** `cargo fmt --check` fails on code that has never been formatted, and CI does not run it. Format only files you add or rewrite (`rustfmt --edition 2021 <file>`) and never reformat the rest as a side effect.
- **Optional cards:** `CardVisibility::shown()` is the one list of the 8 optional cards and fixes their animation slots (`N_OPTIONAL`; TOP CPU = 6, TOP MEM = 7). A new card adds a field there and a slot, never another hand-written bool array. A new card's serde default is `false`, so upgrading doesn't grow the window.
- **Logging:** default filter `warn,bc_computer_monitor=info`. Log lifecycle and saved-state changes at `info` and anything per sample at `debug`. `the_info_filter_names_this_crate` pins the crate name. Settings go through `write_atomic` and log what changed. The `session.running` marker is written after the single-instance check and cleared in `on_exit`.
- **`windows` + `wmi` must be updated together** — transitive version coupling.
- **Minimal edits** — prefer targeted changes; do not refactor surrounding code unless the task requires it.

## After this session

Run **`/wrapup`** (`c:\_repos\Obsidian\Notes\Claude\Skills\wrapup\SKILL.md`). It is the procedure; this section only adds to it. This repo's project note: `Projects/BS-ComputerMonitor Claude Context.md`.

# Copilot Instructions

This repository follows the shared Rust desktop conventions documented in the Obsidian vault.

## Shared reference

Obsidian vault note: `c:\_repos\Obsidian\Notes\Claude\Reference\Rust-Desktop-Standards.md`

Also: `c:\_repos\Obsidian\Notes\Claude\Reference\Author-Version-Standards.md`

## Working rules

- Prefer small, targeted edits over broad rewrites.
- Match the repository's existing style and architecture.
- Treat the shared Rust desktop standards as the default unless this repo has a documented exception.
- Keep egui-family crate versions aligned when changing dependencies.
- Preserve Windows desktop packaging assumptions unless the task explicitly changes them.
- Use focused validation after edits: narrow tests, `cargo check`, or a targeted build step.

## When changing config

- Keep release profile settings intentional.
- Keep theme defaults consistent with the documented standard unless the repo note says otherwise.
- Do not introduce new dependencies without a concrete reason.

## Project-specific overrides

- **Do not add `#![deny(unsafe_code)]`** — 25+ unsafe blocks are required for Windows ETW, DX11, and WGC API calls.
- **`windows` crate and `wmi` must stay at aligned versions** — currently `windows = "0.57"`, `wmi = "0.13"`. Update both together or not at all.
- **egui/eframe pinned at `0.29`** — do not upgrade without a concrete reason.
- **sysinfo pinned at `0.32`** — 0.33 has breaking API changes (`RefreshKind::new()` removed, `refresh()` takes a `bool`, `temperature()` returns `Option<Option<f32>>`). Leave at 0.32 until explicitly upgrading.
- **Custom theme struct** — this app uses a per-metric `Theme` struct, not the standard 4-theme `ThemeManager`. Do not replace it with the shared template.
- **Console visible in debug builds** — `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` is intentional; leave it as-is.

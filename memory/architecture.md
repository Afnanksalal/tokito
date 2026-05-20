# Architecture & stack

Cargo workspace, two members:

- **`tokito`** (root crate) — shared library: `auth`, `config`, `connectivity`, `db`, `handlers` (Axum), `models`, `services`, `store` (SQLx), `settings`, `router`. Also ships a thin `tokito` binary that serves the optional Axum HTTP surface — used mainly for tests / non-default deployments.
- **`tokito-native`** (`native/`) — eframe + egui desktop app. Modules: `app/studio/` (dock, panels: build, bom, projects, research, settings, command palette, inspector, place_panel), `editor/` (canvas, tools, interaction, connectivity sync, ERC, sheets), `base_symbols/`, `symbol_format/` (S-expr `.tokito_sym`/`.kicad_sym`), `mcad_viewer/` (3D preview), `ui/` (design tokens, widgets, typography, layout, toast). Extra bins: `tokito-symbol-import`, `generate-base-symbols`.

**Key versions / choices (as of 2026-05):**

- Rust: `rust-toolchain.toml` declares `channel = "stable"`; CI's `dtolnay/rust-toolchain@1.88` action installs 1.88 but the toolchain file wins, so CI actually builds with whatever stable the runner has. Code is kept lint-clean on **both 1.88 and current stable** — don't rely on a lint or API that only exists in one.
- Edition 2021, `resolver = "2"`. `forbid(unsafe_code)` in library.
- Axum 0.7, Tower-HTTP 0.5, SQLx 0.8 (postgres + tokio + migrate), Tokio (full).
- **`pg-embed` 1.0** with `rt_tokio_migrate` — Postgres is embedded and started in-process; first launch may download binaries.
- Auth: **bcrypt + JWT** (`jsonwebtoken`), local user model; secrets in OS keychain (`keyring` crate).
- HTTP integrations: `reqwest` (rustls), used for AI providers + Firecrawl + Nexar + LCSC.
- Native UI: **eframe/egui 0.29**, `egui_dock` 0.14, `glam` 0.29, `rfd` (file dialogs), `dark-light`. The native crate disables several clippy lints (`too_many_arguments`, `type_complexity`, etc.) and allows `dead_code`.
- Release profile uses `lto = true`, `codegen-units = 1`, `strip = true`.

Migrations live in **`migrations/`** at the workspace root and are shared by both crates. Integration tests live under **`tests/integration/`** as submodules of one custom-harness binary (`tests/integration/main.rs`) — see [testing-and-ci.md](testing-and-ci.md).

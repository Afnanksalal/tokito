# AGENTS.md

**For AI coding assistants working in this repo.** Read this file at the start of every session before doing anything else. It is the team's shared mental model of Tokito — facts one of us learned that all of us should know.

This is not user-facing documentation; humans have `README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, and `docs/`. This file is the AI's notebook.

---

## How to use this file

**Reading.** First thing every session. Treat the contents below as authoritative for project-shaped facts — they override anything stale in your own per-project agent memory.

**Updating.** Edit this file in-place during normal work. Same commit as the code change that made a fact true or false. The repo is the sync mechanism: `git pull` gets you the team's latest knowledge, `git push` (or PR) gives the team yours.

**When to add a fact.** You learned something non-obvious and verified it: a quirky build flag, a constraint, the *why* behind a decision, an external context that affects the work. Lead with the fact, then optionally `**Why:**` and `**How to apply:**` lines if the reasoning isn't self-evident.

**When to update.** A previously-true fact just became false because of your change, or a version/path/symbol drifted. Fix it in place. For facts the team relied on, append a one-line `# updated YYYY-MM-DD: was X, now Y` at the bottom of the section.

**When to delete.** A section is fully obsolete (feature removed, decision reversed). Delete it. Do not leave tombstones or `# DEPRECATED` blocks — git history is the record. Commit message: `prune: <section> — <reason>`.

**Verify before recommending.** A section that names a specific file, function, flag, or version is a claim that was true *when it was written*. If you're about to act on it (not just describe history), grep / read first and update if drifted.

## What does NOT belong here

- Things `README.md` / `CONTRIBUTING.md` / `docs/ARCHITECTURE.md` / `docs/SETTINGS.md` / `docs/API.md` already explain. Update those instead, and reference them from here.
- Things obvious from skimming the code (file structure, naming, what a function does).
- Anything tied to one developer's machine, name, or env — unless clearly scoped under the env-specific section at the bottom.
- Ephemeral state (today's task, current branch, in-progress TODOs). Those go in your own local agent memory.
- Your own preferences as an agent. Same — local.
- Secrets, tokens, internal URLs.

## Commit hygiene for memory updates

Memory edits go in their own commit, not bundled with unrelated code. Easy to revert, easy to audit. If a memory change *is* part of a code change (the code change made the fact true), bundle them — one PR, the diffs read together.

---

# Tokito — the shared mental model

## Product framing

**Tokito** is a desktop **schematic studio** (not a web app): the user describes a board, AI drafts BOM + schematic + research, the user owns and refines the schematic on a native egui canvas.

**Why:** the product positioning everywhere (README, ROADMAP, ARCHITECTURE) is "AI proposes; you approve" + "local-first." Reviews/changes that drift from those principles (e.g. routing data to a cloud service by default, removing the review step, hiding files from the user's app-data folder) should be flagged.

**How to apply:**
- Default to local-first behavior. AI is **optional** and configured by the user with their own keys.
- The **primary user-facing binary** is `tokito-native` (egui desktop). The `tokito` crate is a library + an optional HTTP test surface — do **not** treat the HTTP API as the primary product surface in user-facing copy.
- North star (per ROADMAP): production-grade EDA workflow with serious ERC/DRC, fab-aware outputs, eventual PCB layout. Schematic editor + AI build flow are "shipped today"; PCB layout, footprint/3D linkage, variants, and partner integrations are horizon.

## Architecture & stack

Cargo workspace, two members:

- **`tokito`** (root crate) — shared library: `auth`, `config`, `connectivity`, `db`, `handlers` (Axum), `models`, `services`, `store` (SQLx), `settings`, `router`. Also ships a thin `tokito` binary that serves the optional Axum HTTP surface — used mainly for tests / non-default deployments.
- **`tokito-native`** (`native/`) — eframe + egui desktop app. Modules: `app/studio/` (dock, panels: build, bom, projects, research, settings, command palette, inspector, place_panel), `editor/` (canvas, tools, interaction, connectivity sync, ERC, sheets), `base_symbols/`, `symbol_format/` (S-expr `.tokito_sym`/`.kicad_sym`), `mcad_viewer/` (3D preview), `ui/` (design tokens, widgets, typography, layout, toast). Extra bins: `tokito-symbol-import`, `generate-base-symbols`.

**Key versions / choices (as of 2026-05):**
- Rust toolchain pinned at **1.88** (`rust-toolchain.toml`, CI uses `dtolnay/rust-toolchain@1.88`).
- Edition 2021, `resolver = "2"`. `forbid(unsafe_code)` in library.
- Axum 0.7, Tower-HTTP 0.5, SQLx 0.8 (postgres + tokio + migrate), Tokio (full).
- **`pg-embed` 1.0** with `rt_tokio_migrate` — Postgres is embedded and started in-process; first launch may download binaries.
- Auth: **bcrypt + JWT** (`jsonwebtoken`), local user model; secrets in OS keychain (`keyring` crate).
- HTTP integrations: `reqwest` (rustls), used for AI providers + Firecrawl + Nexar + LCSC.
- Native UI: **eframe/egui 0.29**, `egui_dock` 0.14, `glam` 0.29, `rfd` (file dialogs), `dark-light`. The native crate disables several clippy lints (`too_many_arguments`, `type_complexity`, etc.) and allows `dead_code`.
- Release profile uses `lto = true`, `codegen-units = 1`, `strip = true`.

Migrations live in **`migrations/`** at the workspace root and are shared by both crates. Tests under **`tests/`** include `api_*` (HTTP), `golden_*` (snapshot), `services_exports`, `db_stability`, `spec_compliance`, `ai_pipeline_fixtures`, `notes_research`, `project_workspace`.

## Data model

Embedded Postgres, schema built from `migrations/` (timestamped SQL files, shared by both crates). Core tables:

- **Catalog**: `manufacturers`, `parts` (unique on `(manufacturer_id, mpn)`, JSONB `attributes` + GIN), `part_offers` (per distributor SKU).
- **Designs**: `designs` (with `owner_user_id`, `project_id`), `schematic_instances` (refdes unique per design), `schematic_nets`, `schematic_pins`, `bom_lines` (qty > 0 check).
- **Editor doc**: `schematic_documents` (per-design JSON blob — the editor-grade document the canvas reads/writes; the normalized graph is derived from it).
- **AI build**: `design_intents` (goal_text ≤ 100k chars + JSONB constraints), `design_research_artifacts` (kind ∈ `firecrawl_scrape | firecrawl_search | manual_note`; content ≤ 500k chars; newest-first by `(design_id, created_at DESC)` index), `design_notes`, research annotations.
- **Auth / quotas / audit**: `users` (bcrypt hash, monthly LLM token quota, daily scrape quota), `api_keys`, `usage_daily`, `agent_runs` (status, iterations, token totals, log JSONB).
- **Projects**: `projects` (slug-unique, `workspace_path`) — a default project UUID `00000000-0000-4000-8000-000000000001` is seeded by the projects migration for backfill.

Triggers: `trg_touch_updated_at` on `parts`/`designs`/`bom_lines`; `trg_touch_design_from_child` bumps `designs.updated_at` when intent or research artifacts change.

**How to apply:** new tables go in a new timestamped migration (don't edit shipped ones — they'll be on user disks). New `kind` values for research artifacts must update the CHECK constraint; the current allowed set has been extended over time (`firecrawl_search` was added by `20260508120000_design_research_kind_firecrawl_search.sql`).

## Settings & AI providers

**Settings file** is the primary config, not `.env`:

- Path: **`%LOCALAPPDATA%\tokito\settings.toml`** on Windows (the OS app-data dir on other platforms).
- A one-time legacy `.env` import is supported (`settings_migrated_from_env` flag in `GeneralSettings`).
- `TOKITO_*` env vars **only fill empty fields** in `settings.toml`; they don't disable built-ins. Built-ins (always on): OS keychain, Firecrawl incremental build, ERC strict, bus tool, LCSC catalog, BOM auto-add, open/reveal after export.

**AI providers** (`src/config.rs::AiProvider`): `OpenAi`, `Anthropic`, `Gemini`, `Xai`, `Kimi`. `parse()` defaults to `Xai` for unknown strings. Default models hardcoded in `default_model()` — note **these are forward-looking IDs** (`gpt-5.5`, `claude-sonnet-4-5`, `grok-4.3`, `gemini-2.5-flash`, `kimi-k2.6`); verify against the file before quoting, they may have drifted.

**Why:** the most recent provider commit (`7a89b67 Replace xAI with generic AI provider, bump deps`) genericized the AI layer — older docs still mention xAI specifically and legacy `xai_*` / `TOKITO_XAI_*` keys remain as compatibility aliases. Don't reintroduce xAI-specific assumptions when editing AI code.

**How to apply:** when adding settings, document them in `docs/SETTINGS.md` (CONTRIBUTING.md mandates this) and update the `SettingsFile` structs in `src/settings.rs`. Secrets go through `src/secrets.rs` / OS keychain — never read them from `settings.toml` directly in new code.

## Env vars (reference)

Env vars are an **overlay**, not the source of truth — `settings.toml` is. `merge_from_env` (in `src/settings.rs`) only fills *empty* fields. Setting any of the AI-related vars also flips `general.settings_migrated_from_env = true`.

**Runtime / settings overlay** (`src/settings.rs::merge_from_env`):

| Var | Fills | Notes |
|---|---|---|
| `TOKITO_AI_PROVIDER` | `ai.provider` | One of `openai`/`anthropic`/`gemini`/`xai`/`kimi`; unknown → `xai`. |
| `TOKITO_LLM_API_KEY` | `ai.llm_api_key` | Alias: **`TOKITO_XAI_API_KEY`** (legacy, still honored via `or_else`). |
| `TOKITO_LLM_BASE_URL` | `ai.llm_base_url` | Alias: **`TOKITO_XAI_BASE_URL`** (legacy). |
| `TOKITO_FIRECRAWL_API_KEY` | `ai.firecrawl_api_key` | Required for the Build/research pipeline. |
| `TOKITO_EMBEDDED_PORT` | `database.embedded_port` | Parsed as `u16`; silently dropped if non-numeric. |
| `TOKITO_PG_EMBED_VERSION` | `database.pg_embed_version` | Parsed as `u16` (valid: `16`/`17`/`18`). |
| `TOKITO_LCSC_ANONYMOUS_SEARCH` | `catalog.lcsc_anonymous_search` | Truthy: `1`/`true`/`yes` (case-insensitive). Other values leave the field as-is. |
| `TOKITO_NEXAR_CLIENT_ID` | `catalog.nexar_client_id` | |
| `TOKITO_NEXAR_CLIENT_SECRET` | `catalog.nexar_client_secret` | |

`src/config_provider.rs::86` calls `env::remove_var("TOKITO_XAI_API_KEY")` after applying it, so the legacy var is one-shot consumed (avoids it sticking around in child processes).

**HTTP binary** (`src/main.rs`):

| Var | Effect |
|---|---|
| `TOKITO_STATIC_DIR` | If set + dir + `index.html`, the Axum router serves an SPA fallback for non-`/v1` GETs. Trimmed; empty string treated as unset. |
| `RUST_LOG` | Standard `tracing_subscriber` env-filter; defaults to `tokito=info,tower_http=info` if unset. |

**Test harness** (`src/test_support.rs`):

| Var | Effect |
|---|---|
| **`TOKITO_RUN_DB_INTEGRATION=1`** | Required to run the embedded-Postgres integration suite. Also auto-enabled when `GITHUB_ACTIONS=true`. |
| `TOKITO_TEST_EMBEDDED_PORT` | Override the port the test cluster binds to. |

**Secrets are NOT env vars.** Production keys (LLM, Firecrawl, Nexar) belong in the **OS keychain** via `src/secrets.rs` + the `keyring` crate. Env vars are for CI / dev shells / one-shot bootstrap; reading them at runtime in new code is a smell.

**What is *not* an env var:** `http_addr`, `jwt_secret`, `cors_origins`, agent limits, theme, ERC strict, bus tool, BOM auto-add, export open/reveal — these are only read from `settings.toml` (`SettingsFile`) and the derived `Config`. There is no `TOKITO_HTTP_ADDR` or `TOKITO_JWT_SECRET` overlay despite older docs sometimes implying one.

## HTTP API surface (optional)

The HTTP layer (`src/router.rs`, handlers under `src/handlers/`) is **explicitly secondary**: `docs/API.md` says end users run `Tokito.exe` and don't call HTTP. The HTTP binary is used for automated tests + non-default deployments.

Shape: `GET /health` + `/v1/*`. `/v1/auth/{register,login,api-keys}` is public; the rest is **JWT-protected** via `auth::middleware::require_auth`. Routes cover manufacturers, parts, designs, intent, research (scrape/search/notes/annotate), BOM (get/put/append), schematic graph (`/schematic`, `/schematic/document`, `/schematic/validate`, `/schematic/suggest` — the AI build entrypoint), agent runs (`POST /v1/agent/run`), and integration proxies (`/v1/integrations/{firecrawl,ai,xai}/...` — `xai` path is retained as a back-compat alias for `ai`).

When `TOKITO_STATIC_DIR` is set, the router can fall back to a static SPA build, but the main binary defaults to no UI. CORS origins come from config.

**How to apply:** when changing schemas exposed over `/v1`, also update `docs/API.md` per CONTRIBUTING. Don't pitch new features as web-API-first; the canonical surface is the native studio.

## UI design language

**Stack — pure-Rust native UI, no web layer.** Critical to know before suggesting changes:

- **`eframe` 0.29** = app shell (window, event loop, GL context).
- **`egui` 0.29** = immediate-mode GUI — every frame is redrawn; there is **no retained widget tree, no virtual DOM, no CSS, no JSX**. Idioms from React/Vue/Tauri **do not apply**.
- **`egui_dock` 0.14** = the studio panel docking (Build/BOM/Inspector/Research tabs).
- **`egui_extras` 0.29** = tables (BOM, parts lists in `native/src/ui/table.rs`).
- **`glam` 0.29** = canvas/wire geometry math. **`rfd` 0.15** = native file dialogs (the reason `libgtk-3-dev` is a Linux build dep — **not** because the UI uses GTK widgets). **`open` 5** = "reveal in folder". **`dark-light` 2** = OS theme detection.
- Rendering backend: **glow** (OpenGL) via `egui_glow 0.29` + winit + glutin. **No wgpu in the workspace.**
- The schematic canvas (`native/src/canvas.rs`, `native/src/editor/render.rs`, `native/src/symbols_draw.rs`) draws using egui's `Painter` primitives — lines, rects, circles, text — not an external 2D lib.
- The 3D MCAD preview (`native/src/mcad_viewer/raster.rs`) is a **CPU rasterizer** that hands an image texture to egui. That's why it survives the WSLg software-GL setup.
- It is **not** Tauri, **not** a webview, **not** GTK/Qt/QML, **not** SwiftUI/WPF. Don't propose React/Tailwind/shadcn/Tauri solutions for the desktop UI.

**Shell:** `eframe` window titled "Tokito" (1400×900 default). On Windows the binary uses `windows_subsystem = "windows"` (no console). Entry: `native/src/main.rs` → `app::App` (`native/src/app/mod.rs`).

**Studio layout** (`native/src/app/studio/layout.rs`):

- Far-left fixed 52 px **CAD tool rail** (select, wire, label, hierarchical port, power, junction, no-connect, bus, text, pan — keys Q/W/K/N/H etc.).
- Left **Place panel** and right **Properties/Inspector** are conditional on screen width: place needs ≥ 220 px side budget, inspector needs ≥ 460 px and `properties_panel_open`. Center dock has a 360 px min to avoid `egui_dock` panic on zero-width nodes.
- Bottom 26 px status bar shows cursor X/Y, hovered net, zoom %, active tool. Compacted under 900 px width.
- Panels under `native/src/app/studio/`: `build.rs`, `bom.rs`, `projects.rs`, `research.rs`, `settings.rs`, `inspector.rs`, `place_panel.rs`, `command_palette.rs` (Ctrl+Shift+P), `console.rs`, `messages.rs`, `viewer3d.rs`, `agent.rs`, `design_manager.rs`, `shortcuts.rs`.

**Design tokens** (`native/src/ui/tokens.rs::UiTokens`): teal accent (`#148476`), orange selection (`#E07820`), light gray canvas, wire colors (default/highlight/selected), schematic-ink palette. Default values are light-themed; theme switching is wired via `theme.rs` + `dark-light` crate. Spacing scale `xs=4 / sm=10 / md=16`, radii 6 / 8, symmetric 14×12 panel margin.

**Editor model** (`native/src/editor/`): orthogonal pin-anchored wiring, live union-find connectivity rebuild (`src/connectivity/`), multi-sheet w/ hierarchical labels, ERC markers (live light + full on-demand), undo/redo, wire push/reroute on drag/rotate/mirror, hit-test, junctions, label placement, golden netlist export.

### egui 0.29 idioms & footguns

Researched 2026-05-19 (sources: egui 0.29.1 docs.rs `Ui` / `Layout`, github.com/emilk/egui discussions #469 / #1409, issues #1996 / #1702, `egui_demo_lib` widget_gallery, rerun's `re_ui` crate).

**Footguns this codebase hits:**

1. **`ui.set_width(w)` / `ui.set_max_width(w)` do NOT constrain children.** They only set the parent's `max_rect`; a widget that reports a larger desired size still gets it and the parent silently expands. From the `Ui` docs: *"If a new widget doesn't fit within the `max_rect` then the Ui will make room for it by expanding both `min_rect` and `max_rect`."* Emil's own note in discussion #469: these helpers are "a bit under-developed." Real-world manifestation: `native/src/app/studio/projects.rs` allocates a 260 px right column with `set_width`/`set_max_width` but `secondary_button("Export project zip")` and friends overflow past the window edge.
2. **`horizontal_wrapped` is for inline chips/breadcrumbs, not stacks of full-width buttons.** Wrapping picks one-per-row when needed, but each child still claims its desired width — so a column-of-buttons inside `horizontal_wrapped` still bleeds. See issue #1996.
3. **Custom card helpers** (e.g. `crate::ui::layout::content_card`) should wrap `egui::Frame::group(ui.style())` rather than reinvent stroke/fill — `Frame::group` picks up the active visuals so light/dark themes Just Work.

**Idioms to reach for instead:**

- **Force-fill the cross axis:** `ui.allocate_ui_with_layout(vec2(w, ui.available_height()), Layout::top_down(Align::Min).with_cross_justify(true), |ui| ...)`. `with_cross_justify(true)` is the blessed way to make children stretch to the column width. From the `Layout` docs: *"for vertical layouts justify means all widgets get maximum width."*
- **Per-widget exact size:** `ui.add_sized([w, 0.0], Button::new("..."))`. Allocates the rect *before* the widget asks for its size, so the widget cannot overflow. Canonical per discussion #469.
- **Top-level multi-column layout:** use `SidePanel::left` / `SidePanel::right` (with `.resizable(true).default_width(...)`) + a `CentralPanel` for the flex middle, rather than hand-rolling three nested `ui.vertical` columns inside one `CentralPanel`. That's how rerun and the egui demo are structured.
- **Equal columns:** `ui.columns(n, |cols| { cols[0]. ... })` auto-divides available width. Right tool for even thirds; wrong tool for fixed-left + flex-middle + fixed-right.
- **Responsive breakpoints:** egui has no built-in responsive system. Manual `if ui.available_width() < THRESHOLD` is fine for *hiding* a side panel; don't hand-roll widths for the panels themselves — let `SidePanel`/`columns` do the math.
- **Vertical nav lists** (e.g. project list, sheet list): vertical layout + `selectable_value` (or `SelectableLabel`) one per row, with `Layout::top_down(Align::Min).with_cross_justify(true)` so each row's clickable target spans the full panel width. `horizontal_wrapped` + `selectable_label` is wrong for vertical nav.
- **Spacing & headings live in `style`, not call sites.** Spacing (`item_spacing`, `button_padding`, `window_margin`) and named text styles (`style.text_styles["h2"]`) should be set once at app startup (`setup_custom_style`) and consumed everywhere; avoid sprinkling `ui.add_space(N)` with magic numbers. Rerun's `re_ui` crate is the reference for centralised styling.
- **Empty states:** `Frame::none().inner_margin(24.0)` + `Layout::top_down(Align::Center)` + `ui.add_space(ui.available_height() * 0.3)` above three lines (weak heading, small description, primary CTA). Reference: rerun's "no recording loaded" screen.

**How to apply:**

- Don't add UI controls that flip built-in defaults (ERC strict, bus tool, etc.) — those are intentional product-level constants, not user settings.
- Respect the existing 52 px tool rail width and panel breakpoints when adding chrome; egui_dock panics on zero-width center nodes, so any new side panel needs to obey the `sides_budget` math.
- New panels should plug into the dock via `studio_dock.rs` rather than spawning their own top-level windows.
- When fixing layout bleed, reach for `add_sized` or `allocate_ui_with_layout(... with_cross_justify(true))`; do **not** add more `set_width`/`set_max_width` calls — they don't do what they look like they do.

## Testing & CI

**Local pre-PR check** (per CONTRIBUTING.md, the audit scripts, and CI):

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**DB integration tests** are gated by env var **`TOKITO_RUN_DB_INTEGRATION=1`** because they spin up embedded Postgres (first run downloads binaries). The Make target `make test-db` and `scripts/audit.ps1` enumerate the exact suite:

`api_designs`, `api_parts`, `api_schematic`, `golden_document`, `golden_netlist_move`, `services_exports`, `spec_compliance`, `db_stability`, `notes_research`, `project_workspace`, `ai_pipeline_fixtures`.

CI (`.github/workflows/ci.yml`): single `ubuntu-latest` job, `dtolnay/rust-toolchain@1.88`, installs gtk3/x11/xcb/wayland/xkbcommon deps, then runs `cargo check --locked` → fmt → clippy → unit tests → integration tests with `TOKITO_RUN_DB_INTEGRATION=1`. Concurrency cancels in-progress runs on the same ref. There is **no Windows CI job** despite Windows being the packaged target — keep that in mind when adding platform-specific code.

The `tokito` lib exposes a `test-support` feature; `dev-dependencies` include `tokito = { path = ".", features = ["test-support"] }`, so test helpers (`Config::for_tests`, `AppState::test`, fixtures in `src/test_support.rs`) are gated behind it.

**How to apply:** when adding tests that need a DB, add them to the explicit list in the Makefile + `scripts/audit.ps1` + `.github/workflows/ci.yml` — there's no glob; they're enumerated by name.

## Docs reference (where the canonical human docs live)

Keep these updated per CONTRIBUTING when behavior changes:

- `README.md` — install/run, settings overview, shortcuts.
- `ROADMAP.md` — vision, shipped today vs. horizon (PCB layout, fab DRC, agents).
- `CONTRIBUTING.md` — workspace shape, pre-PR commands, doc-update obligations, MIT licensing.
- `SECURITY.md` — vuln disclosure.
- `docs/ARCHITECTURE.md` — system overview, mermaid diagrams, native module map.
- `docs/SCHEMATIC_EDITOR.md` — editor capabilities, tools, module map.
- `docs/SETTINGS.md` — `settings.toml` reference + always-on built-ins.
- `docs/API.md` — optional HTTP surface route map.

Scripts: `scripts/audit.ps1` (Windows audit), `scripts/package-windows.ps1` (release packaging into `dist/Tokito/`), `scripts/test.ps1`, `Makefile` (`dev`, `test`, `test-db`, `lint`, `fmt`, `check`).

Live upstream remote (as of 2026-05-19): **github.com/VtronTokito/tokito** (SSH). `Cargo.toml`'s `repository` / `homepage` fields still list the old `Afnanksalal/tokito` URL — stale metadata, not the live remote. MIT license.

---

# Env-specific notes

> ⚠️ The sections below are scoped to a particular dev environment, not the project as a whole. Read only the ones that apply to your setup; ignore the rest.

## Linux / WSL2 / WSLg

**Confirmed working on 2026-05-19** on WSL2 (Ubuntu, kernel 5.15 microsoft-standard-WSL2, WSLg present — `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir`).

**The command that works:**

```
WINIT_UNIX_BACKEND=x11 WAYLAND_DISPLAY="" LIBGL_ALWAYS_SOFTWARE=1 ./target/debug/tokito-native
```

**What fails (don't bother retrying without the env overrides above):**

- Plain `cargo run -p tokito-native` → eframe crashes with `winit EventLoopError: Exit Failure: 1`, preceded by `libEGL warning: failed to get driver name for fd -1` / `MESA: error: ZINK: failed to choose pdev` / `libEGL warning: egl: failed to create dri2 screen`. WSLg's Wayland path picks the zink GL→Vulkan adapter and falls over.
- `LIBGL_ALWAYS_SOFTWARE=1 WGPU_BACKEND=gl` **alone** is not enough — winit still tries Wayland first and fails with broken-pipe spam before any frame paints.

**Why the workaround:** with `WAYLAND_DISPLAY` cleared and `WINIT_UNIX_BACKEND=x11`, winit talks to the WSLg X server via `DISPLAY=:0`; `LIBGL_ALWAYS_SOFTWARE=1` makes Mesa use llvmpipe instead of the broken zink adapter.

**Known cosmetic issue:** under software GL the studio UI layout looks "off". Functionality works; this is a WSLg software-renderer artifact, not a code bug. Native Linux with a real GPU + Windows packaged build should not show this.

**First-launch side effect:** `tokito::db::embedded` downloads pg-embed Postgres 16 binaries into `~/.cache/pg-embed/linux/amd64/16.12.0/` (a few minutes; needs internet). Subsequent launches are instant.

**Apt deps that had to be installed manually** (the CI list minus what Ubuntu already has): `libxcb-shape0-dev`, `libxcb-xfixes0-dev`. The other 5 (`libgtk-3-dev libx11-dev libxcb-render0-dev libxkbcommon-dev libwayland-dev`) were already present.

**How to apply:**

- When running the desktop binary on WSL2, go straight to the X11+software-GL env vars; don't burn time on the default path.
- `scripts/run-linux.sh` (debug `--check` / `--release` / `--package`) is in place but **does not** set these env vars itself. Either export them in the shell first or extend the script.
- The cosmetic UI issue is **not** a bug to fix in code; flag any "fix the layout" requests as likely a real-GPU problem instead.

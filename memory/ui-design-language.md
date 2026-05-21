# UI design language

## `tokito_ui` — the shared component library

The projects/landing experience is built on **`tokito_ui`**, a separate egui
component library — repo **github.com/VtronTokito/ui**, consumed by `native/`
as a dependency (path dep on the landing branch; git dep on `master`).

- **What it owns:** the `Tokens` palette (dark/light), theme application,
  Phosphor icon helpers, and composable primitives — `card`, `new_tile`,
  `icon_button`, `text_button`, `link`, `list_row`, `text_input`,
  `search_field`, `toggle`, `modal`, `page_header`, `section_header`.
- **How to use it:** `use tokito_ui::components as c;` then `c::card(ui, &t, …)`
  etc., with a `tokito_ui::Tokens` value (`Tokens::from_name(&theme)`).
- **Where it's used:** `native/src/app/studio/projects.rs` (projects home +
  per-project designs view) is fully ported to it. The studio/editor still
  uses the app's older `crate::ui::*` (`UiTokens`, `widgets`, `layout`) — a
  gradual migration; new chrome should prefer `tokito_ui`.
- **Rules** (see the repo's `AGENTS.md`): primitives not finished widgets;
  every component is a free fn taking `&Tokens`; widgets with internal state
  take an explicit `id_source`; pin egui 0.29 + egui-phosphor 0.7.x together.
- **Two token types coexist:** `tokito_ui::Tokens` (landing) and the app's
  `crate::ui::tokens::UiTokens` (studio, with schematic-specific colours).
  Don't conflate them.

**Stack — pure-Rust native UI, no web layer.** Critical to know before suggesting changes:

- **`eframe` 0.29** = app shell (window, event loop, GL context).
- **`egui` 0.29** = immediate-mode GUI — every frame is redrawn; there is **no retained widget tree, no virtual DOM, no CSS, no JSX**. Idioms from React/Vue/Tauri **do not apply**.
- **`egui_dock` 0.14** = the studio panel docking (Build/BOM/Inspector/Research tabs).
- **`egui_extras` 0.29** = tables (BOM, parts lists in `native/src/ui/table.rs`).
- **`egui-phosphor` 0.7** = Phosphor icon font (Regular variant). Stay on **0.7.x** — versions ≥ 0.8 target egui ≥ 0.30. See the icon-rendering footgun below.
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

## egui 0.29 idioms & footguns

Researched 2026-05-19 (sources: egui 0.29.1 docs.rs `Ui` / `Layout`, github.com/emilk/egui discussions #469 / #1409, issues #1996 / #1702, `egui_demo_lib` widget_gallery, rerun's `re_ui` crate).

**Footguns this codebase hits:**

1. **`ui.set_width(w)` / `ui.set_max_width(w)` do NOT constrain children.** They only set the parent's `max_rect`; a widget that reports a larger desired size still gets it and the parent silently expands. From the `Ui` docs: *"If a new widget doesn't fit within the `max_rect` then the Ui will make room for it by expanding both `min_rect` and `max_rect`."* Emil's own note in discussion #469: these helpers are "a bit under-developed." Real-world manifestation: `native/src/app/studio/projects.rs` allocates a 260 px right column with `set_width`/`set_max_width` but `secondary_button("Export project zip")` and friends overflow past the window edge.
2. **`horizontal_wrapped` is for inline chips/breadcrumbs, not stacks of full-width buttons.** Wrapping picks one-per-row when needed, but each child still claims its desired width — so a column-of-buttons inside `horizontal_wrapped` still bleeds. See issue #1996.
3. **Custom card helpers** (e.g. `crate::ui::layout::content_card`) should wrap `egui::Frame::group(ui.style())` rather than reinvent stroke/fill — `Frame::group` picks up the active visuals so light/dark themes Just Work.
4. **Hand-computed card rects drift.** `allocate_exact_size` + `painter_at` + `child_ui(manual_rect)` to lay out a card's contents reliably produces visible offsets. Use `egui::Frame` (fill + rounding + stroke + `inner_margin`) and lay contents out with normal top-down flow inside it; never arithmetic on `Rect::from_min_size` for sub-regions of a widget.
5. **Phosphor icons must render via a dedicated font family.** Inter Var has glyphs at some Private-Use-Area codepoints that collide with Phosphor's. If Phosphor is only a *fallback* in the Proportional family, Inter intercepts those codepoints and paints a stray glyph (folder→"m", caret→"▶", dots→"p"). Putting Phosphor *first* breaks all Latin text (its font covers Latin with blank glyphs). **Fix:** `theme.rs` registers a separate `FontFamily::Name("phosphor")` family containing only the icon font; `native/src/ui/icons.rs` renders icons through it (`icons::icon(...)`, `icons::icon_label(...)`, `icons::icon_font(...)`). Never put a Phosphor glyph in a string that also contains Latin rendered by the default family.

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

# Tokito

**Describe the board. AI drafts it. You own the schematic.**

Tokito is a **desktop schematic studio** for electronics: AI gathers datasheets and parts, proposes a BOM and schematic, and you refine everything on a fast native canvas—symbols, wiring, ERC, exports.

## What you get

- **AI-assisted build** — Describe the goal; the app researches parts, grounds the BOM, and proposes a schematic you review before it lands on the canvas.
- **Professional editor** — Place symbols, wire, label nets, run ERC, undo/redo, multi-sheet documents.
- **Your library** — Parts and BOM live in a **local database** on your machine; designs and research stay under your account folder.
- **Exports** — SVG, PDF, netlists, BOM-friendly outputs, and MCAD handoff JSON.
- **Sourcing-aware search** — LCSC and optional Nexar hints while you place parts.

## Windows — install and run

**To use Tokito:** run the packaged app (no separate database setup).

1. Build the folder (requires [Rust](https://rustup.rs/) 1.88+ and [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) — C++ workload):

   ```powershell
   .\scripts\package-windows.ps1
   ```

2. Open `dist\Tokito\`. Copy `.env.example` to `.env` next to `Tokito.exe` and set your keys (required for AI build):

   - `TOKITO_XAI_API_KEY`
   - `TOKITO_FIRECRAWL_API_KEY`

3. Double-click **`Tokito.exe`**. Keep the **`assets`** folder beside the executable.

Design data and the bundled database files live under **`%LOCALAPPDATA%\tokito\`**. The first launch may download database runtime components once (internet needed that time).

## Using Tokito

| Shortcut | Action |
|----------|--------|
| Select | Q |
| Wire | W |
| Pan | H |
| Grid / Snap | Toolbar (G / S) |
| Zoom to fit | Home |
| Undo / Redo | Ctrl+Z / Ctrl+Y |
| AI build (send prompt) | Ctrl/Cmd+Enter |
| Command palette | Ctrl+Shift+P |

Use **Panels** in the menu bar or **Ctrl+Shift+P** → “Board preview” if the tab is closed. The right-hand tab strip can scroll—there is a fourth tab **Preview** after Design.

## Configuration

| Variable | Required | Purpose |
|----------|----------|---------|
| `TOKITO_XAI_API_KEY` | **Yes** for AI build | Planning, parts, schematic draft |
| `TOKITO_FIRECRAWL_API_KEY` | **Yes** for AI build | Research / datasheets |
| `TOKITO_EMBEDDED_PORT` | No | Local database port (default `15432`) |
| `TOKITO_JWT_SECRET` | Release-quality auth | Strong signing for stored sessions (optional in dev) |
| `TOKITO_NEXAR_*` | No | Richer package metadata in search |
| `TOKITO_PG_EMBED_VERSION` | No | `16` / `17` / `18` if the default embedded DB bundle fails on your PC |

Full list: [`.env.example`](.env.example).

## Symbols

Bundled symbols are in [`assets/base-symbols/`](assets/base-symbols/) (see license file in that tree).

## Vision & roadmap

Long-term direction—**Altium-grade workflow**, **KiCad-style freedom**, **Flux-level parallel AI**, from idea toward **production-ready PCB** (layout, routing, fab/assembly partners): see [`ROADMAP.md`](ROADMAP.md).

## License

**MIT** — [`LICENSE`](LICENSE).

## For contributors

Workflow, tests, and repo layout: [`CONTRIBUTING.md`](CONTRIBUTING.md). Deeper internals: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

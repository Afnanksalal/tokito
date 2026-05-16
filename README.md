# Tokito

**Describe the board. AI drafts it. You own the schematic.**

Tokito is a desktop schematic studio: AI gathers datasheets and parts, proposes a BOM and schematic, and you refine everything on a native canvas—symbols, wiring, ERC, and exports.

## What you get

- **AI-assisted build** — Research, BOM grounding, and a schematic proposal you review before it lands on the canvas.
- **Schematic editor** — Library symbols, pin-anchored wiring, live connectivity, multi-sheet designs, ERC. See [docs/SCHEMATIC_EDITOR.md](docs/SCHEMATIC_EDITOR.md).
- **Local library** — Parts and BOM in PostgreSQL under your app-data folder.
- **Exports** — SVG, PDF, netlists, BOM-friendly outputs, MCAD handoff JSON.
- **Sourcing** — LCSC catalog search (on by default); optional Nexar for richer metadata.

## Windows — install and run

1. Build (Rust 1.88+, [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ workload):

   ```powershell
   .\scripts\package-windows.ps1
   ```

2. Copy `.env.example` to `.env` next to `Tokito.exe` in `dist\Tokito\`. Set:

   - `TOKITO_XAI_API_KEY`
   - `TOKITO_FIRECRAWL_API_KEY`

3. Run **`Tokito.exe`**. Keep the **`assets`** folder beside the executable.

Data lives under **`%LOCALAPPDATA%\tokito\`**. First launch may download embedded database binaries once.

## Shortcuts

| Key | Action |
|-----|--------|
| Q | Select |
| W | Wire |
| K | Sheet port (hierarchical) |
| N | Net label |
| H | Pan |
| G / S | Grid / snap (toolbar) |
| Home | Zoom to fit |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl/Cmd+Enter | Send AI build prompt |
| Ctrl+Shift+P | Command palette |

## Configuration

| Variable | Required | Purpose |
|----------|----------|---------|
| `TOKITO_XAI_API_KEY` | For AI build | Planning, parts, schematic draft |
| `TOKITO_FIRECRAWL_API_KEY` | For AI build | Research / datasheets |
| `TOKITO_JWT_SECRET` | Release builds | Session signing (dev default if unset) |
| `TOKITO_LCSC_ANONYMOUS_SEARCH` | No (default on) | LCSC in Place panel |
| `TOKITO_NEXAR_*` | No | Nexar catalog metadata |
| `TOKITO_EMBEDDED_PORT` | No | Postgres port (default `15432`) |
| `TOKITO_PG_EMBED_VERSION` | No | `16` / `17` / `18` if embed fails |

Full list: [`.env.example`](.env.example).

## Symbols

Bundled libraries: [`assets/base-symbols/`](assets/base-symbols/) (see [LICENSE](assets/base-symbols/LICENSE.md)). Import external `.tokito_sym` or `.kicad_sym` trees via **Place → Import symbol library**.

## Docs & contributing

- [ROADMAP.md](ROADMAP.md) — Vision and horizon
- [CONTRIBUTING.md](CONTRIBUTING.md) — Build, test, PR guidelines
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — System overview
- [SECURITY.md](SECURITY.md) — Vulnerability reporting

## License

MIT — [LICENSE](LICENSE).

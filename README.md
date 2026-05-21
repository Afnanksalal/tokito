# Tokito

**Describe the board. AI drafts it. You own the schematic.**

Tokito is a desktop schematic studio: AI gathers datasheets and parts, proposes a BOM and schematic, and you refine everything on a native canvas: symbols, wiring, ERC, and exports.

## What you get

- **AI-assisted build**: Research, BOM grounding, and a schematic proposal you review before it lands on the canvas.
- **Schematic editor**: Library symbols, pin-anchored wiring, live connectivity, multi-sheet designs, ERC. See [docs/SCHEMATIC_EDITOR.md](docs/SCHEMATIC_EDITOR.md).
- **Local library**: Parts and BOM in PostgreSQL under your app-data folder.
- **Exports**: SVG, PDF (plot + pack), netlists, BOM CSV, MCAD handoff JSON, project bundles.
- **Sourcing**: LCSC catalog search (on by default); optional Nexar for richer metadata.

## Windows - install and run

1. Build (Rust 1.88+, [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ workload):

   ```powershell
   .\scripts\package-windows.ps1
   ```

2. Run **`Tokito.exe`**. Keep the **`assets`** folder beside the executable.

3. Open **Settings** in Studio, choose an AI provider, and enter that provider key plus your **Firecrawl** key. No `.env` file is required for normal use.

Data lives under **`%LOCALAPPDATA%\tokito\`** (`settings.toml`, projects, embedded Postgres). First launch may download database binaries once.

## Configuration (Settings-first)

Primary config: **`%LOCALAPPDATA%\tokito\settings.toml`** (edited in the Studio **Settings** tab).

| Settings section | Purpose |
|------------------|---------|
| General | Theme, default export format |
| Database | Embedded Postgres port, version, data directory |
| AI | Provider, provider key, Firecrawl key, model limits |
| Catalog | Optional Nexar credentials |
| Advanced | HTTP/JWT when running the server binary |

Built-in by default: **OS keychain**, **Firecrawl incremental build**, **ERC strict**, **bus tool**, **LCSC catalog**, **BOM auto-add**, **open/reveal after export**. See [docs/SETTINGS.md](docs/SETTINGS.md). A one-time import from legacy **`.env`** is supported.

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

## Symbols

Bundled libraries: [`assets/base-symbols/`](assets/base-symbols/) (see [LICENSE](assets/base-symbols/LICENSE.md)). Import external `.tokito_sym` or `.kicad_sym` trees via **Place > Import symbol library**.

## Docs & contributing

- [Project board](https://github.com/orgs/VtronTokito/projects/1): Roadmap, status, and active work
- [CONTRIBUTING.md](CONTRIBUTING.md): Build, test, PR guidelines
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): System overview
- [docs/SETTINGS.md](docs/SETTINGS.md): `settings.toml` reference
- [docs/API.md](docs/API.md): HTTP layer (reads the same `settings.toml` when run as a service)
- [SECURITY.md](SECURITY.md): Vulnerability reporting

Run **`.\scripts\audit.ps1`** for fmt, clippy, tests, and `cargo audit`.

## License

MIT - [LICENSE](LICENSE).

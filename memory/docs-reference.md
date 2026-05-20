# Docs reference (where the canonical human docs live)

Keep these updated per CONTRIBUTING when behavior changes:

- `README.md` — install/run, settings overview, shortcuts.
- `ROADMAP.md` — vision, shipped today vs. horizon (PCB layout, fab DRC, agents).
- `CONTRIBUTING.md` — workspace shape, pre-PR commands, doc-update obligations, MIT licensing.
- `SECURITY.md` — vuln disclosure.
- `docs/ARCHITECTURE.md` — system overview, mermaid diagrams, native module map.
- `docs/SCHEMATIC_EDITOR.md` — editor capabilities, tools, module map.
- `docs/SETTINGS.md` — `settings.toml` reference + always-on built-ins.
- `docs/API.md` — optional HTTP surface route map.

Scripts: `scripts/audit.ps1` (Windows audit — fmt/clippy/tests/cargo-deny), `scripts/package-windows.ps1` (release packaging into `dist/Tokito/`), `scripts/test.ps1`, `Makefile` (`dev`, `test`, `test-db`, `lint`, `fmt`, `deny`, `check`).

Repo: **github.com/VtronTokito/tokito** (MIT). `Cargo.toml`'s `repository` / `homepage` fields point here.

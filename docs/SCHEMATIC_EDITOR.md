# Schematic editor

The native studio canvas is a full schematic editor backed by the shared `tokito` connectivity engine.

## Capabilities

- **Library symbols** — Bundled and user `.tokito_sym` libraries; import `.kicad_sym` via Place → Import symbol library.
- **Placement** — Symbols tab, parts catalog, LCSC/Nexar search with automatic part rows in the local database.
- **Wiring** — Orthogonal wires anchored to pins; junctions at crossings; net names from labels and power symbols.
- **Connectivity** — Live union-find rebuild on edit; stable `net_id`; net highlight and ERC markers.
- **Tools** — Select, wire, net label, hierarchical sheet port, power, junction, no-connect, bus, text, pan.
- **Editing** — Undo/redo, copy/paste, rotate/mirror with wire reroute, multi-select move, wire push on drag.
- **Sheets** — Multi-sheet documents with hierarchical labels (`child_sheet/net`).
- **Validation** — Live ERC (light) while editing; full ERC on demand before export.

## Module map

| Path | Role |
|------|------|
| `native/src/editor/` | Canvas, tools, interaction, connectivity sync |
| `native/src/base_symbols/` | Symbol library load and paint |
| `native/src/symbol_format/` | S-expression symbol files |
| `src/connectivity/` | Shared net graph rebuild |

## Exports

Schematic geometry flushes to `SchematicDocument` JSON, then netlists (S-expression, connectivity text), SVG, PDF, and MCAD handoff JSON.

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the editor fits the desktop app.

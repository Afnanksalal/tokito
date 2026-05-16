# Tokito roadmap

Where the product is headed—not a fixed schedule. Priorities change as we ship and learn.

## North star

**Production-grade EDA workflow** — serious electrical rules, fab-aware output, and data you own on disk.  
**AI in parallel** — research, BOM, and draft schematics while you stay in control of what ships.

## Principles

- **AI proposes; you approve** — Nothing reaches fabrication without explicit review.
- **Local-first** — Designs, parts, and research live in your database and app-data folder.
- **Interop** — Open exports and handoff to PCB layout, MCAD, and assembly partners.

## Shipped today

- Native **schematic editor** — Library symbols, pin-anchored wires, live connectivity, ERC, multi-sheet, hierarchical ports, LCSC catalog placement.
- **AI-assisted build** — Prompt → research → BOM → schematic proposal → review → apply.
- **Exports** — SVG, PDF, netlists, MCAD handoff JSON.
- **Symbol import** — `.tokito_sym` and `.kicad_sym` into the user library.

## Horizon

### Schematic & library

- Stronger ERC and DRC before layout.
- Footprint and 3D model linkage per part.
- Variant and assembly options.

### PCB layout

- Placement, copper, planes, interactive and batch routing.
- Fab DRC/DFM with clear failure explanations.

### AI & automation

- Background agents for routing passes, BOM scrubbing, alternates.
- Cost and lead-time aware suggestions under user policies.

### Production

- Fab and assembly partner integrations (quotes, uploads, BOM sync).
- Revision discipline and MOQ/lead-time in the workflow.

### Platform

- Large-design performance; optional collaboration without losing local ownership.

---

See [CONTRIBUTING.md](CONTRIBUTING.md) to help shape direction.

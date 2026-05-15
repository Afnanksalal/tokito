# Schematic editor plan

Tokito's current native canvas is useful as an MVP graph editor, but it is not yet a production schematic editor. The target is a pin-aware CAD surface with the editing primitives users expect from KiCad, the polish and contextual panels of Altium, and Tokito's grounded copilot workflow as the differentiator.

This document turns the editor research into an implementation plan.

---

## Product target

Tokito should feel like a real schematic capture tool, not a dashboard with draggable cards.

The editor should support:

- Accurate schematic primitives: symbols, pins, wires, wire segments, junctions, net labels, power symbols, no-connect markers, buses, text, and sheet frames.
- Pin-aware connectivity: wires connect to pins and junctions, not just one component reference designator to another.
- Engineering workflow: ERC, net highlighting, symbol/part properties, BOM sync, footprint assignment, export, and AI actions.
- Fast CAD interaction: hotkeys, icon toolbars, command palette, snap/grid controls, box select, multi-select, drag, rotate, mirror, duplicate, delete, undo/redo.
- Production UI: dense, predictable, high-contrast canvas, contextual inspectors, diagnostics, library browser, and polished empty/loading/error states.

Non-goal for the next milestone: full PCB layout. The schematic editor should prepare for PCB handoff and KiCad/netlist export, but PCB routing is not the first rebuild target.

---

## Reference tools

### KiCad

Primary behavioral reference. KiCad's schematic editor is built around symbols, wires, labels, junctions, buses, power symbols, ERC, symbol libraries, net highlighting, hierarchy, BOM/netlist generation, and PCB transfer.

Apply to Tokito:

- Copy the primitive set: symbol, pin, wire, bus, junction, no-connect, local/global/hierarchical labels, power symbols.
- Copy CAD mechanics: right-side drawing tools, left display controls, grid/snap, hotkeys, box select, selection filter, net highlight.
- Copy schematic validity expectations: a schematic is a logical design entry point, not just a picture.

Source: https://docs.kicad.org/9.0/en/eeschema/eeschema.html

### Altium Designer

Primary UI polish reference. Altium relies heavily on a contextual Properties panel and Messages panel that change based on selected document/object.

Apply to Tokito:

- Use one contextual inspector instead of many one-off forms.
- Show document settings when nothing is selected.
- Show symbol, pin, wire, net, label, BOM, and part fields when selected.
- Keep validation feedback in a bottom diagnostics/messages panel.

Sources:

- https://www.altium.com/documentation/altium-designer/schematic
- https://www.altium.com/documentation/altium-designer/properties-panel

### Flux

Primary AI-native reference. Flux combines browser-based schematic editing, project context, parts, and an AI assistant that can inspect and change the design.

Apply to Tokito:

- Make copilot actions visible as grounded design operations: add part, place symbol, connect net, explain ERC, find alternative, attach datasheet.
- Keep research artifacts and BOM provenance next to the editor, not hidden in logs.
- Let the user approve AI edits before they land on the canvas.

Source: https://docs.flux.ai/flux/Introduction/getting-started-in-flux--schematic

### EasyEDA

Practical manufacturing/catalog reference. EasyEDA emphasizes libraries, design manager, DRC, multi-sheet work, footprints, and catalog/manufacturing integration.

Apply to Tokito:

- Add a Design Manager panel listing sheets, components, nets, warnings, and outputs.
- Treat the parts catalog as a first-class placement source.
- Keep LCSC/Nexar/offer integrations aligned with schematic/BOM editing.

Source: https://docs.easyeda.com/en/Introduction/UI-Introduction/index.html

### Autodesk Fusion Electronics

Workflow reference for integrated schematic, simulation, PCB, and manufacturing framing.

Apply to Tokito:

- Keep the long-term flow explicit: schematic -> ERC -> BOM/parts -> netlist/KiCad/PCB.
- Avoid disconnected export-only workflows where possible.

Source: https://www.autodesk.com/solutions/circuit-design-software

---

## Current gap

Current native canvas state is intentionally simple:

- `native/src/canvas.rs` has `Sym { ref_des, part_id, pos, rotation_deg }`.
- `Wire { a, b, net }` connects one symbol refdes to another symbol refdes.
- `src/models/schematic.rs` stores instances, nets, and logical pins, but no rendered wire geometry, labels, junctions, sheet frame, or symbol pin placement.

This means Tokito cannot yet represent a true schematic drawing. It can say "U1 pin VDD is on net VCC", but the canvas cannot faithfully draw or edit the path, label, junction, or pin endpoint as CAD geometry.

Therefore, UI polish alone will not solve the editor. The editor rebuild should start with the schematic document model.

---

## Proposed schematic document model

Keep `ReplaceSchematic` compatible for API callers, but introduce an internal/editor-grade document model that can round-trip real schematic geometry.

Core objects:

- `SchematicDocument`
  - `sheets`
  - `symbols`
  - `wire_segments`
  - `junctions`
  - `net_labels`
  - `power_symbols`
  - `no_connects`
  - `text_items`
  - `buses`
  - `erc_markers`
- `Sheet`
  - `id`, `name`, `path`, `page_size`, `title_block`, `origin`, `grid`
- `SymbolInstance`
  - `id`, `part_id`, `symbol_id`, `ref_des`, `value`, `position`, `rotation`, `mirror`, `fields`, `footprint_ref`
- `SymbolDefinition`
  - `id`, `name`, `body`, `pins`, `units`, `source`
- `SymbolPin`
  - `number`, `name`, `electrical_type`, `position`, `orientation`, `visible`
- `WireSegment`
  - `id`, `sheet_id`, `start`, `end`, `net_id`, `style`
- `NetLabel`
  - `id`, `sheet_id`, `name`, `kind`, `position`, `orientation`
- `Junction`
  - `id`, `sheet_id`, `position`
- `NoConnect`
  - `id`, `sheet_id`, `position`

Connectivity should be derived from geometry plus labels, then normalized into the existing `schematic_nets` and `schematic_pins` tables for API/export compatibility.

Database migration options:

- Short term: store editor geometry in `schematic_instances.meta` plus a new `design_schematic_documents` JSONB table. This reduces migration risk while the editor matures.
- Long term: normalize geometry into tables once the object model stabilizes.

Recommended short-term table:

```sql
CREATE TABLE design_schematic_documents (
    design_id UUID PRIMARY KEY REFERENCES designs(id) ON DELETE CASCADE,
    document_json JSONB NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

## UI layout

### Main frame

- Top app bar: project/design name, save state, Run ERC, Export, Generate, command palette.
- Left rail: icon-only CAD tools.
- Left dock: Project / Library / Design Manager.
- Center: schematic canvas.
- Right dock: contextual Properties / Copilot / Research.
- Bottom dock: Messages / ERC / Console.
- Status bar: cursor coordinates, relative coordinates, grid, zoom, active tool, selection count.

### Left CAD tools

Use icons with tooltips:

- Select
- Place symbol
- Wire
- Net label
- Power symbol
- Junction
- No-connect
- Bus
- Text
- Pan

Tool modes should stay active until Escape, matching CAD expectations.

### Canvas visual standard

- Light canvas option should be first-class; most schematics are easier to read on off-white or very dark neutral, not decorative tinted panels.
- Grid should be subtle and stable across zoom.
- Symbols should use thin schematic strokes, not rounded cards.
- Wires should be orthogonal by default.
- Selection should be visible without making the symbol look like a UI card.
- Net highlight should color all connected segments, labels, and pins.
- Pin endpoints should have hover affordances.
- Junction dots should only appear where electrically meaningful.

### Contextual inspector

When nothing is selected:

- Sheet name
- Page size
- Grid/snap
- ERC profile
- Display options

When a symbol is selected:

- Refdes
- Value/comment
- MPN/manufacturer
- Part ID
- Footprint
- Datasheet/research links
- Symbol fields
- Rotation/mirror/unit
- Pin list with nets

When a wire/net is selected:

- Net name
- Net class
- Connected pins
- Labels
- ERC warnings

When ERC marker is selected:

- Severity
- Message
- Related objects
- Suggested fix
- Ask copilot action

### Copilot panel

The copilot should not just dump generated output. It should offer reviewable actions:

- Add these parts to BOM.
- Place these symbols on sheet.
- Connect these pins/nets.
- Add missing power/no-connect markers.
- Explain this ERC warning.
- Find replacement part.

Every action should show provenance: BOM line, part, research artifact, or model inference.

---

## Editor behavior

### Placement

- Symbol placement snaps to grid.
- Pin endpoints snap independently to wires and labels.
- Press `R` to rotate, `X`/`Y` to mirror while placing or after selection.
- Auto-annotation is optional but should be available.
- New symbols should create stable refdes values.

### Wiring

- Orthogonal wire mode by default.
- Segment editing by dragging endpoints and bends.
- Click pin -> click pin should route Manhattan segments.
- Wire-to-wire crossing is not connected unless there is a junction.
- Wire endpoint touching another wire can create a junction.
- Net labels connect same-name nets according to scope rules.

### Selection

- Single click selects the closest object.
- Shift adds to selection.
- Drag left-to-right selects fully enclosed objects.
- Drag right-to-left selects touched objects.
- Selection filter can restrict to symbols, wires, labels, junctions, text, or ERC markers.
- Double click opens the relevant inspector focus.

### Validation

Run lightweight validation continuously and full ERC on demand/save:

- Unconnected required pins.
- Power input without driver.
- Output-output conflicts.
- Passive-only net warnings where useful.
- Single-pin nets.
- Duplicate refdes.
- Missing footprint.
- Symbol part mismatch.
- Label dangling/unattached.
- Sheet pin/hierarchical label mismatch once hierarchy is added.

### Persistence

Save should persist both:

- Editor document geometry.
- Normalized schematic graph for API/export/BOM/copilot compatibility.

The normalized graph should be derived, not hand-edited separately.

---

## Implementation milestones

### Milestone 1: Editor document foundation

Goal: represent real schematic geometry without replacing the whole UI at once.

- Add `SchematicDocument` Rust structs. **Implemented.**
- Add JSONB persistence for `design_schematic_documents`. **Implemented.**
- Add conversion from existing `SchematicView` to document geometry. **Implemented.**
- Add conversion from document geometry to `ReplaceSchematic`. **Implemented.**
- Keep existing API behavior unchanged. **Implemented; normalized schematic routes still work.**

Acceptance:

- Existing generated schematics open in the new document model.
- Saving still writes the existing normalized graph.
- No data loss for existing designs.

### Milestone 2: Pin-aware canvas

Goal: stop connecting refdes-to-refdes.

- Render symbols from `SymbolDefinition` pins.
- Add pin hit-testing. **Initial native support implemented.**
- Add wire segments. **Initial native support implemented with Manhattan segments.**
- Add labels, junctions, no-connect markers. **Initial native support implemented.**
- Add power symbols, buses, and text items. **Initial native placement, selection, inspector, undo/delete, and JSONB round-trip implemented.**
- Add net derivation from geometry. **Implemented in the shared document model, including labels/junctions/pins on wire segment interiors.**
- Add net highlighting. **Initial native support implemented for selected wires and labels.**

Acceptance:

- User can place two symbols and connect exact pins.
- Wires render as orthogonal segments.
- Selecting a net highlights all connected geometry.
- Save produces correct `schematic_pins`.

### Milestone 3: Production CAD shell

Goal: make the app feel like a schematic editor.

- Redesign the studio layout around CAD conventions.
- Replace text-heavy buttons with icon toolbars and tooltips.
- Expand the icon rail beyond MVP wiring tools: select, wire, label, power, junction, no-connect, bus, text, pan, focus, fit, grid, snap. **Implemented in native.**
- Add status bar with coordinates/grid/zoom/tool.
- Add Design Manager panel.
- Add contextual Properties panel.
- Add Messages/ERC panel.

Acceptance:

- No card-like component blocks on the canvas.
- Core actions are reachable by icon and hotkey.
- Selection context changes the inspector.
- ERC messages can navigate to objects.

### Milestone 4: Library and placement workflow

Goal: parts and symbols become first-class editing sources.

- Add symbol library browser.
- Map parts to symbols and footprints.
- Add power symbol chooser.
- Add footprint assignment field.
- Add part placement from catalog/BOM/research.

Acceptance:

- User can search a part and place its symbol.
- User can place generic R/C/L/diode/transistor/op-amp/connector/power symbols.
- BOM and placed symbols stay linked by `part_id`.

### Milestone 5: Copilot as reviewable edit operations

Goal: AI edits become inspectable CAD actions.

- Make Generate return proposed editor operations.
- Show a diff/action list before applying.
- Link proposed actions to parts, BOM lines, and research artifacts.
- Add focused copilot actions for ERC fixes and part replacement.

Acceptance:

- User can approve/reject individual AI edits.
- Applied edits are undoable.
- Copilot never silently overwrites user geometry.

### Milestone 6: Hierarchy and export hardening

Goal: support larger designs and downstream tooling.

- Add multi-sheet and sheet navigation.
- Add local/global/hierarchical label semantics.
- Add KiCad netlist/export path.
- Add PDF/SVG plotting.
- Add test fixtures for generated and manually edited schematics.

Acceptance:

- Multi-sheet designs validate correctly.
- Exported netlist matches derived connectivity.
- Plots are readable and suitable for review.

---

## Near-term technical decisions

Recommended:

- Keep Rust/egui for now if the goal is incremental delivery.
- Do not start with a frontend rewrite unless there is a separate product decision to build a browser app.
- Move canvas state out of `App` into a dedicated editor document/controller module.
- Treat rendering, hit-testing, connectivity derivation, and persistence as separate layers.
- Add golden tests for document -> normalized graph conversion.

Possible module split:

```text
native/src/editor/
  document.rs       # SchematicDocument structs
  geometry.rs       # points, segments, bounds, transforms
  connectivity.rs   # derive nets from geometry/labels
  hit_test.rs       # object picking
  commands.rs       # undoable edit commands
  render.rs         # egui painter code
  tools.rs          # select/wire/place/label state machines
  inspector.rs      # contextual property models
```

Shared model candidates:

```text
src/models/schematic_document.rs
src/services/schematic_document.rs
src/store/schematic_document.rs
```

---

## Quality bar

The rebuild should be judged against editing behavior, not screenshots.

Minimum production bar:

- Pin-level editing works.
- Wiring is orthogonal and predictable.
- Selection is precise on dense schematics.
- Undo/redo covers every editing command.
- ERC warnings are visible, navigable, and understandable.
- Save/reload round-trips geometry.
- Generated schematics are editable without losing BOM/part provenance.
- No component on the schematic canvas looks like a UI card.

---

## Risks

- A visual redesign before the document model will produce another polished mock editor, not a real CAD tool.
- Over-normalizing geometry tables too early will slow iteration.
- AI-generated schematics will remain frustrating unless the copilot emits or can be converted into precise pin-level geometry.
- Full KiCad compatibility is a large scope. Start with export/netlist compatibility before trying to import/export every schematic feature.

---

## First implementation slice

Start with a narrow vertical slice:

1. Add `SchematicDocument` JSONB persistence.
2. Convert current `Sym` and `Wire` data into document objects.
3. Render pin-aware generic symbols for R/C/L/U/J/P.
4. Replace `Wire { a, b, net }` with `WireSegment`.
5. Derive `ReplaceSchematic` from pins/segments/labels.
6. Add net highlight and contextual inspector for symbol/net.

This slice will make Tokito cross the line from "visual graph editor" into "schematic editor foundation" without blocking on every advanced EDA feature.

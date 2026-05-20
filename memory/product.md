# Product framing

**Tokito** is a desktop **schematic studio** (not a web app): the user describes a board, AI drafts BOM + schematic + research, the user owns and refines the schematic on a native egui canvas.

**Why:** the product positioning everywhere (README, ROADMAP, ARCHITECTURE) is "AI proposes; you approve" + "local-first." Reviews/changes that drift from those principles (e.g. routing data to a cloud service by default, removing the review step, hiding files from the user's app-data folder) should be flagged.

**How to apply:**

- Default to local-first behavior. AI is **optional** and configured by the user with their own keys.
- The **primary user-facing binary** is `tokito-native` (egui desktop). The `tokito` crate is a library + an optional HTTP test surface — do **not** treat the HTTP API as the primary product surface in user-facing copy.
- North star (per ROADMAP): production-grade EDA workflow with serious ERC/DRC, fab-aware outputs, eventual PCB layout. Schematic editor + AI build flow are "shipped today"; PCB layout, footprint/3D linkage, variants, and partner integrations are horizon.

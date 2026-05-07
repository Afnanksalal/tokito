# Product plan

Tokito aims at **copilot-grade hardware design**: the user states intent; the system **grounds** outputs in **stored research** and a **real parts catalog**; schematics stay **editable** and **exportable** like real CAD workflows.

This document tracks **stages**, **quality bars**, and what is **already shipped** in this repository.

---

## Stage 1 — Design intent & persistence ✅

**Goal:** Treat “what we’re building” as first-class data on each design.

**Shipped**

- Table **`design_intents`**: `goal_text`, `constraints_json`, timestamps.
- **`GET/PUT /v1/designs/:id/intent`** with validation (length caps, JSON object for constraints).
- Native **prompt field** persists intent and feeds the copilot pipeline.

**Quality bar:** Ownership checks mirror other design routes; migrations stay reversible in spirit.

---

## Stage 2 — Research artifacts & ingestion ✅

**Goal:** Every model-facing **fact** should be **traceable** to a stored artifact.

**Shipped**

- **`design_research_artifacts`** with `kind`, URLs, markdown/text, metadata.
- **`GET /v1/designs/:id/research`**, **`POST …/research/scrape`**, **`POST …/research/search`**.
- **`research_pipeline`** normalizes Firecrawl responses into artifacts.
- **`kind`** values: `firecrawl_scrape`, **`firecrawl_search`**, `manual_note` (see migration widening search).

**Native UX:** **Generate** runs **planned** Firecrawl queries from the xAI step inside **`design_pipeline`**—no separate mandatory “search button” before generate.

**Quality bar:** Scrape quotas + structured errors; truncation recorded in metadata when applicable.

---

## Stage 3 — Grounded schematic generation ✅

**Goal:** Schematics tied to **BOM `part_id`s** and **research excerpts**, not prompt-only JSON.

**Shipped**

- **`design_pipeline::build_design_from_prompt`** — full orchestration through BOM replace.
- **`schematic_gen`** strict mode when BOM populated; topology validation before accepting payloads.
- **`POST …/schematic/suggest`** and native **Generate** share this path.

**Quality bar:** **`validate_topology`** blocks bad graphs; excerpt/token budgets enforced; pipeline fails clearly if Firecrawl yields nothing usable.

---

## Stage 4 — Canvas editor foundations ✅

**Goal:** Users can **fix** AI output interactively.

**Shipped (native)**

- Grid snap (~40 px), **rotation**, **undo/redo** for schematic edits.
- Persisted rotation in store.

**Quality bar:** Predictable shortcuts; capped undo stack for memory safety.

---

## Stage 5 — Electrical validity beyond topology 🔄

**Goal:** “Valid” means useful **ERC**, not only graph shape.

**Shipped (light)**

- Topology errors + **`erc_light`** warnings (floating stubs, unused nets, etc.) on save, suggest, validate.

**Next**

- Pin electrical classes, richer ERC tiers.

---

## Stage 6 — 3D & MCAD 🔮

**Goal:** Industrial **3D** needs footprints and placement—typically via **KiCad** export path or a dedicated viewer.

**Shipped (partial)**

- **`GET …/export?format=netlist`** for tooling bridges.

**Out of scope** until ERC/model hardening stabilizes.

---

## Operational checklist (ongoing)

- Run migrations in CI / locally before merge when schema changes.
- Handlers: **`assert_visible`** on design scope.
- Logs: useful correlation IDs; **never** log secrets or raw JWTs.
- Keep **native** and **HTTP** behavior aligned through shared **store** + **services**.

---

## Implementation snapshot

| Stage | Theme | Status |
|-------|--------|--------|
| 1 | Intent | ✅ Shipped |
| 2 | Research artifacts | ✅ Shipped |
| 3 | Grounded schematic + pipeline | ✅ Shipped |
| 4 | Canvas UX | ✅ Shipped (native) |
| 5 | ERC depth | 🔄 Light ERC live; deeper rules planned |
| 6 | 3D / KiCad | 🔮 Netlist export; viewer TBD |

---

## References

- Firecrawl Search: [docs.firecrawl.dev](https://docs.firecrawl.dev/features/search)  
- Practical interchange: **KiCad** ecosystem for long-term “hard” validity  

Update this file when you **complete** or **reprioritize** a stage.

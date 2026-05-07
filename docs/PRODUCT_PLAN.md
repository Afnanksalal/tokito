# Tokito product plan — copilot-grade hardware design

This document is the authoritative phased roadmap for turning Tokito into a **production-grade intent-driven hardware copilot**: user states what to build; the system **grounds** answers in scraped datasheets and catalog data; the **schematic** is validated and **editable** like a real CAD tool.

---

## Stage 1 — Design intent & persistence *(implemented in codebase iteration)*

**Goal:** Capture “what the user wants to build” as first-class data tied to a design.

**Deliverables:**

- `design_intents` table: `goal_text`, structured `constraints_json` (JSON — rails, current, topology hints).
- API: `GET/PUT /v1/designs/:id/intent` with validation (max lengths, JSON object only).
- Native UI: prompt field doubles as **intent** (saved before schematic generation).

**Quality bar:** migrations reversible mindset; ownership checks match other design routes; `designs.updated_at` touched when intent changes.

---

## Stage 2 — Research artifacts & ingestion

**Goal:** Every factual claim the model uses should be **traceable** to stored artifacts.

**Deliverables:**

- `design_research_artifacts` table: `kind`, `source_url`, `content_text`, `metadata_json`, timestamps.
- API: `GET /v1/designs/:id/research`, `POST /v1/designs/:id/research/scrape` (batch URLs via Firecrawl server-side; quotas enforced per URL).
- API: `POST /v1/designs/:id/research/search` — Firecrawl [**Search**](https://docs.firecrawl.dev/features/search) (`query` + optional limits); markdown scraped per hit (`scrapeOptions` applied server-side). **Native:** Firecrawl search is **not** a separate manual step — **Generate** runs planned queries from the xAI plan inside `design_pipeline` (then BOM + schematic).
- Service `research_pipeline`: normalize Firecrawl JSON → markdown/text excerpt + metadata.

**Quality bar:** scrape quota + structured errors; no silent truncation without recording metadata.

---

## Stage 3 — Grounded schematic generation

**Goal:** Schematics are **catalog-backed** and **traceable** to research + BOM, not prompt-only hallucination.

**Deliverables:**

- **`design_pipeline::build_design_from_prompt`** orchestrates: intent → xAI plan (queries + candidates) → **mandatory** Firecrawl web search into artifacts → xAI part resolution → manufacturers/parts + **BOM replace** → `schematic_gen::suggest_from_prompt` with **strict BOM `part_id`s** when the BOM is populated.
- `schematic_gen::assemble_grounding_context` feeds design metadata, intent, BOM enrichment, capped research excerpts.
- System prompts prefer datasheet excerpts; topology validation before accepting JSON.

**Quality bar:** deterministic **topology validation** (`schematic_validate::validate_topology`) before accepting model JSON; token budget guards on excerpt size; pipeline fails if Firecrawl cannot ingest research text.

---

## Stage 4 — Canvas editor foundations

**Goal:** Users can **correct** AI output like in ECAD tools.

**Deliverables:**

- Grid snap (e.g. 40 px), symbol **rotation**, discrete **undo/redo** for schematic graph edits.
- Persist rotation in schematic store.

**Quality bar:** predictable shortcuts (Ctrl+Z / Ctrl+Y); history capped to avoid memory growth.

---

## Stage 5 — Validity beyond graph shape *(next major milestone)*

**Goal:** “Valid schematic” means **electrical rules**, not only unique refdes/nets.

**Deliverables:**

- Pin-level model: map `pin_name` → electrical class (power, GND, IO, NC…) per part or heuristic rules.
- ERC passes: single GND net discipline hints, obvious shorts, undriven outputs (tiered warnings).

**Quality bar:** ERC runs on save and on AI preview before apply.

---

## Stage 6 — 3D & MCAD *(parallel track)*

**Goal:** True **3D** requires footprints + placement — typically KiCad STEP or proprietary viewer.

**Options (pick one later):**

- Export **KiCad** + use KiCad 3D viewer; or
- Embed **wgpu** scene with simplified extruded footprints (heavy engineering).

**Quality bar:** tie 3D to footprint library — out of scope until Stage 5 stabilizes.

---

## Operational checklist (every stage)

- Migrations tested locally and in CI integration job when applicable.
- Handlers: auth + `assert_visible` on `design_id`.
- Logs: user/design IDs at info where helpful; never log API keys or raw JWT.
- Native + HTTP API remain behaviorally aligned (same stores/services).

---

## References (external landscape)

- Multi-agent / NL → schematic research (e.g. CircuitLM-style pipelines with ERC).
- OSS CAD direction: KiCad interoperability remains the pragmatic route for “industrial validity.”

This file should be updated when a stage is **completed** or **reprioritized**.

---

## Implementation log (repository)

| Stage | Status |
|-------|--------|
| **1 — Intent** | Landed: `design_intents`, `GET/PUT …/intent`, native editor + DB persistence. |
| **2 — Research artifacts** | Landed: `design_research_artifacts`, `GET …/research`, `POST …/research/scrape`, `POST …/research/search`, `research_pipeline` service. |
| **3 — Grounded schematic_gen + pipeline** | Landed: `design_pipeline` (plan → Firecrawl → BOM → schematic); `schematic_gen` strict mode for catalog `part_id`s; HTTP `POST …/schematic/suggest` and native **Generate** use the full pipeline. |
| **4 — Canvas foundations** | Landed (native): 40 px grid snap, rotation persisted, undo/redo stack (Ctrl+Z / Ctrl+Y), index-based interaction loop for correct borrows. |
| **5 — ERC** | Landed (light): topology hard-errors + duplicate `(instance_ref, pin_name)`; `erc_light` warnings (single-pin nets, unused nets, instances without pins, multiple ground-like nets). Runs on DB save, AI draft, `POST …/schematic/validate`. Full pin electrical classes → future. |
| **6 — 3D** | Partial: text **`GET …/export?format=netlist`** connectivity export for tooling / KiCad-adjacent workflows; native STEP/viewer still TBD. |

Export JSON (`GET …/export`) now embeds `intent` and `research_artifacts` alongside BOM/schematic.

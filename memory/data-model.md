# Data model

Embedded Postgres, schema built from `migrations/` (timestamped SQL files, shared by both crates). Core tables:

- **Catalog**: `manufacturers`, `parts` (unique on `(manufacturer_id, mpn)`, JSONB `attributes` + GIN), `part_offers` (per distributor SKU).
- **Designs**: `designs` (with `owner_user_id`, `project_id`), `schematic_instances` (refdes unique per design), `schematic_nets`, `schematic_pins`, `bom_lines` (qty > 0 check).
- **Editor doc**: `schematic_documents` (per-design JSON blob — the editor-grade document the canvas reads/writes; the normalized graph is derived from it).
- **AI build**: `design_intents` (goal_text ≤ 100k chars + JSONB constraints), `design_research_artifacts` (kind ∈ `firecrawl_scrape | firecrawl_search | manual_note | annotation`; content ≤ 500k chars; newest-first by `(design_id, created_at DESC)` index), `design_notes`, research annotations.
- **Auth / quotas / audit**: `users` (bcrypt hash, monthly LLM token quota, daily scrape quota), `api_keys`, `usage_daily`, `agent_runs` (status, iterations, token totals, log JSONB).
- **Projects**: `projects` (slug-unique, `workspace_path`) — a default project UUID `00000000-0000-4000-8000-000000000001` is seeded by the projects migration for backfill.

Triggers: `trg_touch_updated_at` on `parts`/`designs`/`bom_lines`; `trg_touch_design_from_child` bumps `designs.updated_at` when intent or research artifacts change.

**How to apply:** new tables go in a new timestamped migration (don't edit shipped ones — they'll be on user disks). New `kind` values for research artifacts must update the CHECK constraint; the current allowed set has been extended over time (`firecrawl_search` was added by `20260508120000_design_research_kind_firecrawl_search.sql`; `annotation` was added by `20260619120000_research_annotation.sql`).

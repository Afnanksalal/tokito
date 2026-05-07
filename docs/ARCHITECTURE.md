# Architecture

Tokito splits into a **Rust library + Axum server**, an optional **egui native shell**, and **PostgreSQL** as the system of record. This page summarizes how requests flow and where state lives.

---

## Runtime surfaces

| Surface | Crate / binary | Role |
|---------|------------------|------|
| **Desktop studio** | `tokito-native` | egui UI, SQLx pool, calls into `tokito` as a library—no HTTP to self required for editing |
| **HTTP API** | `tokito` binary | Same domain logic; serves `/v1`, `/health`, optional static SPA (`TOKITO_STATIC_DIR`) |

Both binaries **run SQLx migrations** from **`migrations/`** at startup (`sqlx::migrate!`), keeping schema and code in sync for single-developer and small-team deployments.

---

## Request path (HTTP)

```
Client → Axum router (CORS, trace)
      → Auth middleware where applicable
      → Handler (validation)
      → Store (SQLx) and/or Service (integrations)
      → PostgreSQL / external APIs (xAI, Firecrawl, …)
```

- **`src/router.rs`** — route table, **`AppState`** (pool, HTTP client, integration configs).
- **`src/handlers/`** — HTTP adapters; map **`AppError`** to JSON.
- **`src/store/`** — queries and transactions (`bom::replace_validated`, `schematic::replace`, …).
- **`src/services/`** — orchestration and vendors:
  - **`design_pipeline`** — copilot: plan → Firecrawl → resolve parts → BOM → schematic.
  - **`research_pipeline`** — Firecrawl search/scrape → **`design_research_artifacts`**.
  - **`schematic_gen`** / **`schematic_validate`** — LLM draft + topology & ERC.
  - **`xai`**, **`firecrawl`**, **`agent`**, offers sync, etc.

---

## Copilot data flow (conceptual)

```
User prompt
    → intent persisted
    → xAI: queries + candidate MPNs
    → Firecrawl: artifacts rows (search/scrape)
    → xAI: resolved BOM lines
    → Postgres: manufacturers, parts, bom_lines
    → xAI: ReplaceSchematic (part_id ∈ BOM)
```

Research excerpts and BOM lines are concatenated into the **grounding context** for schematic generation so pin names and topology align with stored catalog rows.

---

## Data model (PostgreSQL)

| Area | Tables (conceptual) |
|------|---------------------|
| Identity / quotas | Users, API keys, usage counters (see migrations) |
| Catalog | **`manufacturers`**, **`parts`** (unique `(manufacturer_id, mpn)`), **`part_offers`** |
| Design container | **`designs`** |
| Intent | **`design_intents`** (`goal_text`, `constraints_json`) |
| Research | **`design_research_artifacts`** (`kind`, `source_url`, `content_text`, …) |
| Schematic | **`schematic_instances`**, **`schematic_nets`**, **`schematic_pins`** |
| Procurement | **`bom_lines`** (editable aggregate; may diverge from canvas until unified) |

Foreign keys enforce visibility and cascade rules per migration files.

---

## Operational notes

- **Pooling:** `TOKITO_DB_MAX_CONNECTIONS` caps the SQLx pool.
- **Horizontal scale:** Keep the API **stateless**; use one logical Postgres (read replicas only with explicit read routing if added later).
- **Migrations at boot:** Fine for MVP; larger deployments may prefer external migration jobs and a feature flag to disable auto-migrate.
- **Secrets:** Never commit `.env`; production requires strong **`TOKITO_JWT_SECRET`** and TLS in front of the API.

---

## Related reading

- **[API.md](API.md)** — route-level detail  
- **[PRODUCT_PLAN.md](PRODUCT_PLAN.md)** — roadmap vs. shipped features  

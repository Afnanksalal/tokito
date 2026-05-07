# Tokito architecture

## Layers

1. **HTTP (Axum)** — `src/router.rs` wires routes and middleware (CORS, HTTP tracing). Shared state is `AppState` (`router.rs`) holding a PostgreSQL pool (`sqlx::PgPool`).
2. **Handlers** — `src/handlers/` validate inputs and call the store. Errors map to JSON via `AppError` in `src/error.rs`.
3. **Services** — `src/services/` holds integrations and orchestration: **Firecrawl** (`research_pipeline`, `firecrawl`), **xAI** (`xai`, `schematic_gen`), **`design_pipeline`** (canonical flow: plan → web search → resolve parts → BOM → schematic with catalog `part_id`s), validation (`schematic_validate`), agent, offers sync, etc.
4. **Store** — `src/store/` contains SQLx queries and transactions. Heavy writes (BOM replace, schematic replace) run in a single transaction.
5. **Persistence** — PostgreSQL. Schema is versioned in `migrations/` and applied at startup.

## Data model (summary)

- **manufacturers** → **parts** (unique MPN per manufacturer). **part_offers** holds optional distributor rows (future pricing sync).
- **designs** are containers for **schematic_instances** (refdes, optional `part_id`, placement), **schematic_nets**, and **schematic_pins** (instance pin → net).
- **bom_lines** are a separate aggregate for procurement; can be derived from the schematic later or edited independently.

## Operational notes

- Migrations run on every process start using embedded SQL (`sqlx::migrate!`). For large fleets, prefer external migration jobs and optionally disable automatic migrate behind a flag in a future revision.
- Connection pooling is configured via `TOKITO_DB_MAX_CONNECTIONS`.
- For horizontal scaling, make the API stateless and use a single logical Postgres (or replicas for read scaling with explicit routing in application code).

# Tokito

**Tokito** is an electronics design MVP: **parts catalog**, **schematics** (instances, nets, pins), and **BOM** lines, backed by **PostgreSQL**.

The primary delivery is the **`tokito-native`** desktop app (**Rust + egui**): Postgres-backed studio with schematic canvas, catalog search, and **prompt-driven generation** that **grounds** schematics in **web research + catalog parts**. You can run **API-only** mode (`cargo run -p tokito`) for automation or servers.

> **Replace `YOUR_ORG/tokito`** in the badge URL after you publish the GitHub repository.

[![CI](https://github.com/YOUR_ORG/tokito/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_ORG/tokito/actions/workflows/ci.yml)

## Copilot pipeline (native & API suggest)

**Generate** / **`POST /v1/designs/:id/schematic/suggest`** runs one orchestrated pipeline (not prompt-only schematic JSON):

1. **Intent** — user prompt stored on the design (`design_intents`).
2. **Plan (xAI)** — search queries + candidate MPNs for the topology.
3. **Firecrawl web search** — each planned query runs server-side; results are saved as **`design_research_artifacts`** (markdown excerpts).
4. **Resolve (xAI)** — normalized parts + quantities from excerpts + candidates.
5. **Catalog + BOM** — manufacturers/parts upserted in Postgres; BOM replaced with validated `part_id`s.
6. **Schematic (xAI)** — draft `ReplaceSchematic` using **only BOM `part_id`s** (no null catalog IDs when the BOM is populated).

**Required env:** `TOKITO_XAI_API_KEY` and **`TOKITO_FIRECRAWL_API_KEY`** for this path. See `.env.example`.

Additional endpoints (`research/scrape`, `research/search`, Firecrawl proxies) remain available for tooling or manual ingestion.

## What you get

- REST API under `/v1` for manufacturers, parts, designs, BOM, schematic graph, intent, research artifacts, integrations, agent, and offers.
- SQL migrations embedded in the binary (`sqlx::migrate!`).
- Structured logging via `tracing`.
- CORS configurable for browser clients.
- Docker Compose for local Postgres; multi-stage `Dockerfile` for deployment.
- CI: `fmt`, `clippy -D warnings`, `cargo test`, optional Postgres integration test.

## Requirements

- Rust **1.74+** (stable).
- PostgreSQL **14+** (CI uses 16).
- **Docker** (optional, for Compose).

## Quick start (native desktop)

1. Copy environment:

   ```bash
   cp .env.example .env
   ```

2. Start Postgres:

   ```bash
   docker compose up -d postgres
   ```

3. Set `TOKITO_DATABASE_URL` in `.env`.

4. For AI generation, set **`TOKITO_XAI_API_KEY`** and **`TOKITO_FIRECRAWL_API_KEY`**.

5. Run the app:

   ```bash
   cargo run -p tokito-native
   ```

See `.env.example` for Nexar, LCSC, agent limits, JWT, and test DB URL.

## Quick start (API-only server)

1. Follow steps 1–4 above (database + keys as needed).

2. Run migrations + HTTP API:

   ```bash
   cargo run -p tokito
   ```

   Listens on `TOKITO_HTTP_ADDR` (default `0.0.0.0:8080`).

3. Health check:

   ```bash
   curl -s http://localhost:8080/health
   ```

Optional: serve a static SPA from disk:

```bash
TOKITO_STATIC_DIR=/path/to/dist cargo run -p tokito
```

## Configuration

| Variable | Description |
|----------|-------------|
| `TOKITO_HTTP_ADDR` | Bind address (default `0.0.0.0:8080`). |
| `TOKITO_DATABASE_URL` | **Required.** Postgres URL, e.g. `postgres://tokito:tokito@localhost:5433/tokito?sslmode=disable`. |
| `TOKITO_DB_MAX_CONNECTIONS` | Pool size (default `10`). |
| `TOKITO_CORS_ORIGINS` | Comma-separated origins; empty = permissive CORS (dev-friendly). |
| `TOKITO_STATIC_DIR` | Optional. Directory containing a built SPA. Enables `/app` client routing on the API server. |

Full list: `.env.example` (xAI, Firecrawl, Nexar, LCSC, agent, JWT, integration tests).

## API overview

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Liveness / JSON status. |
| GET/POST | `/v1/manufacturers` | List / create. |
| GET/POST | `/v1/parts` | Search / create part. |
| GET | `/v1/parts/:id` | Part by ID. |
| POST | `/v1/designs` | Create design. |
| GET/PATCH | `/v1/designs/:id` | Get / patch metadata. |
| GET/PUT | `/v1/designs/:id/bom` | List / replace BOM. |
| GET/PUT | `/v1/designs/:id/schematic` | Get / replace schematic graph. PUT returns `{ ok, erc_warnings }`. |
| POST | `/v1/designs/:id/schematic/suggest` | **Full copilot pipeline** → `{ schematic, erc_warnings }` (plan → Firecrawl → BOM → schematic). Requires xAI + Firecrawl. |
| POST | `/v1/designs/:id/schematic/validate` | Non-persisting topology + ERC check. |
| GET | `/v1/designs/:id/export` | Snapshot JSON; `?format=csv` BOM; `?format=netlist` connectivity. |
| GET/PUT | `/v1/designs/:id/intent` | Build goal + `constraints` JSON. |
| GET | `/v1/designs/:id/research` | Research artifacts (newest first). |
| POST | `/v1/designs/:id/research/scrape` | Batch Firecrawl scrape URLs → artifacts. |
| POST | `/v1/designs/:id/research/search` | Firecrawl web search → artifacts. |

Replace BOM: `{ "lines": [ { "part_id", "quantity", "sort_order", "notes" } ] }`.  
Replace schematic: `{ "instances", "nets", "pins" }` — pins use `instance_ref` (refdes) and `net_name`. Details: **`docs/API.md`**.

## Documentation

| Doc | Contents |
|-----|----------|
| [`docs/API.md`](docs/API.md) | HTTP routes, request/response shapes. |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Layers, data model, operations. |
| [`docs/PRODUCT_PLAN.md`](docs/PRODUCT_PLAN.md) | Roadmap and implementation status. |

## Testing

- Default: `cargo test --workspace`
- Integration test (Postgres): set `TOKITO_TEST_DATABASE_URL`, then:

  ```bash
  cargo test -p tokito --test integration -- --ignored --nocapture
  ```

CI runs the same integration job against Postgres 16.

## Project layout

```
tokito/
├── migrations/               # SQLx migrations
├── native/                   # egui desktop (`tokito-native`)
├── src/                      # Library + API (`tokito`): handlers, store, services (incl. design_pipeline)
├── tests/integration.rs      # optional DB integration tests
├── docs/                     # API, architecture, product plan
├── scripts/                  # maintenance helpers
├── Dockerfile
├── docker-compose.yml        # local Postgres (if present)
├── LICENSE-MIT               # MIT license text
├── LICENSE-APACHE            # Apache 2.0 license text
├── CONTRIBUTING.md
└── SECURITY.md
```

## Contributing & security

- **[CONTRIBUTING.md](CONTRIBUTING.md)** — fmt, clippy, tests, PR expectations.
- **[SECURITY.md](SECURITY.md)** — how to report vulnerabilities.

## License

Licensed under **either** of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option (SPDX: **`MIT OR Apache-2.0`**).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Tokito shall be dual-licensed as above, without any additional terms or conditions.

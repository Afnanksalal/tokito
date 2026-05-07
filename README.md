# Tokito

**Grounded hardware design, not prompt fiction.** Tokito is a **desktop studio** and **HTTP API** for **parts**, **BOMs**, and **schematics**—with an AI copilot that **searches the web**, **fills your parts database**, and **places real catalog parts** on the canvas before it drafts connectivity.

[![CI](https://github.com/Afnanksalal/tokito/actions/workflows/ci.yml/badge.svg)](https://github.com/Afnanksalal/tokito/actions/workflows/ci.yml)

---

## Why Tokito

| Problem | How Tokito approaches it |
|--------|---------------------------|
| LLMs invent parts that don’t exist | Pipeline **resolves MPNs** and writes **manufacturers + parts + BOM** in Postgres before schematic JSON |
| No provenance for “datasheet facts” | **Firecrawl** search results stored as **`design_research_artifacts`** (markdown excerpts + URLs) |
| Schematic JSON disconnected from procurement | Every placed instance is tied to **`part_id`** from the BOM when the copilot path runs |

---

## What you can do

- **Describe a circuit** in plain language → run **Generate** (native) or **`POST …/schematic/suggest`** (API).
- **Edit** the graph like a lightweight CAD tool: grid snap, rotate symbols, undo/redo, save to Postgres.
- **Search** your parts catalog and **drop** components onto the canvas.
- **Export** design snapshots (JSON), BOM CSV, or a text netlist for downstream tools.
- **Integrate** via **`/v1`** REST for automation, CI, or a custom UI.

---

## Copilot pipeline (the heart of Generate)

One button runs **six stages** end-to-end—no separate “search first” step in the UI:

1. **Intent** — Your prompt is saved as the design’s build goal (`design_intents`).
2. **Plan (xAI)** — Model proposes Firecrawl **search queries** and **candidate MPNs** for the topology.
3. **Research (Firecrawl)** — Each query runs as **web search**; hits become **`design_research_artifacts`** (excerpt text + source metadata).
4. **Resolve (xAI)** — Model maps excerpts + candidates to **concrete parts** (MPN, manufacturer, qty, notes).
5. **Catalog + BOM** — Rows upserted into **`manufacturers`**, **`parts`**, then **`bom_lines`** replace for this design (validated `part_id`s).
6. **Schematic (xAI)** — Draft **`ReplaceSchematic`** using **only BOM part UUIDs**—no orphan fantasy parts when the BOM is populated.

**Required for this path:** `TOKITO_XAI_API_KEY` and `TOKITO_FIRECRAWL_API_KEY`. Optional **`POST …/research/scrape`** and **`…/research/search`** remain for scripts or manual ingestion.

Details: [`docs/API.md`](docs/API.md) · [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)

---

## Quick start

**Stack:** Rust **1.74+**, PostgreSQL **14+** (CI uses 16). Docker optional for Postgres via [`docker-compose.yml`](docker-compose.yml).

### Desktop app (recommended)

```bash
cp .env.example .env
# Set TOKITO_DATABASE_URL; for Generate also set TOKITO_XAI_API_KEY + TOKITO_FIRECRAWL_API_KEY

docker compose up -d postgres   # optional

cargo run -p tokito-native
```

On first launch the app migrates the schema and provisions a **local single-user** account for offline-style use.

### HTTP API only

```bash
cargo run -p tokito
```

Listens on **`TOKITO_HTTP_ADDR`** (default `0.0.0.0:8080`). Health:

```bash
curl -s http://localhost:8080/health
```

Optional: serve a built SPA from disk:

```bash
TOKITO_STATIC_DIR=/path/to/dist cargo run -p tokito
```

---

## Configuration (high level)

| Variable | Role |
|----------|------|
| `TOKITO_DATABASE_URL` | **Required.** Postgres connection string. |
| `TOKITO_HTTP_ADDR` | API bind address (API binary only). |
| `TOKITO_DB_MAX_CONNECTIONS` | Pool size (default `10`). |
| `TOKITO_JWT_SECRET` | Sign JWTs for `/v1` API auth—**set in production**. |
| `TOKITO_XAI_API_KEY` | Grok / OpenAI-compatible chat for copilot stages. |
| `TOKITO_FIRECRAWL_API_KEY` | Web search + scrape for research artifacts. |
| `TOKITO_CORS_ORIGINS` | Browser origins for the API; empty = permissive dev mode. |
| `TOKITO_STATIC_DIR` | Optional static assets + SPA fallback for `/app`. |

Full reference: [`.env.example`](.env.example) (Nexar, LCSC, agent limits, integration tests).

---

## API at a glance

| Area | Examples |
|------|-----------|
| Catalog | `GET/POST /v1/manufacturers`, `GET/POST /v1/parts`, `GET /v1/parts/:id` |
| Designs | `POST /v1/designs`, `GET/PATCH /v1/designs/:id`, `GET …/export` |
| BOM | `GET/PUT /v1/designs/:id/bom` |
| Schematic | `GET/PUT …/schematic`, **`POST …/schematic/suggest`** (full pipeline), `POST …/schematic/validate` |
| Copilot data | `GET/PUT …/intent`, `GET …/research`, `POST …/research/scrape`, `POST …/research/search` |

Authoritative request/response shapes: **[`docs/API.md`](docs/API.md)**

---

## Repository layout

```
tokito/
├── native/           # egui desktop — tokito-native
├── src/              # Library + API — handlers, store, services (design_pipeline, schematic_gen, …)
├── migrations/       # SQLx migrations (applied on startup)
├── docs/             # API, architecture, roadmap
├── tests/            # Integration tests (Postgres)
├── Dockerfile
├── docker-compose.yml
├── LICENSE-MIT · LICENSE-APACHE
├── CONTRIBUTING.md · SECURITY.md
└── README.md         # you are here
```

---

## Testing & CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Integration test (requires Postgres + `TOKITO_TEST_DATABASE_URL`):

```bash
cargo test -p tokito --test integration -- --ignored --nocapture
```

GitHub Actions runs format, clippy, unit tests, and the integration job against Postgres 16.

---

## Contributing & security

- **[CONTRIBUTING.md](CONTRIBUTING.md)** — workflow, PR checklist, license on contributions.
- **[SECURITY.md](SECURITY.md)** — responsible disclosure (avoid public issues for undisclosed vulns).

---

## Tech stack

- **Rust**, **Axum**, **SQLx**, **PostgreSQL**, **Tokio**
- **egui** / **eframe** for the native shell
- **xAI** (OpenAI-compatible) · **Firecrawl** for search/scrape

---

## License

Dual-licensed under **MIT** ([`LICENSE-MIT`](LICENSE-MIT)) or **Apache 2.0** ([`LICENSE-APACHE`](LICENSE-APACHE)), at your option (**SPDX:** `MIT OR Apache-2.0`).

Unless you say otherwise, contributions you submit are accepted under those same terms.

---

## More documentation

| Doc | Contents |
|-----|----------|
| [`docs/README.md`](docs/README.md) | Index of technical docs |
| [`docs/API.md`](docs/API.md) | REST reference |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Layers, data model, ops |
| [`docs/PRODUCT_PLAN.md`](docs/PRODUCT_PLAN.md) | Roadmap and delivery stages |

# Architecture

Tokito is a Rust library and Axum server with an optional egui native shell. **Embedded PostgreSQL** (pg-embed) is the system of record — no external or cloud database.

## Runtime surfaces

| Surface | Binary | Role |
|---------|--------|------|
| Desktop | `tokito-native` | egui studio; links `tokito` for HTTP-backed features |
| API | `tokito` | `/v1`, `/health`, optional static files (`TOKITO_STATIC_DIR`) |

```mermaid
flowchart TB
  subgraph desktop[Desktop shell]
    N[tokito-native]
  end
  subgraph server[HTTP server]
    B[tokito binary]
  end
  subgraph core[Shared library]
    L[tokito crate<br/>router · handlers · store · services]
  end
  subgraph data[Local data]
    PG[(Embedded PostgreSQL<br/>pg-embed)]
  end
  N --> L
  B --> L
  L --> PG
```

Both binaries run SQLx migrations from `migrations/` at startup.

## HTTP request path

```mermaid
flowchart LR
  C[Client] --> A[Axum<br/>CORS · trace]
  A --> AM[Auth middleware]
  AM --> H[Handler]
  H --> S[Store<br/>SQLx]
  H --> SV[Service layer<br/>external APIs]
  S --> DB[(Embedded PostgreSQL)]
  SV --> X[xAI]
  SV --> F[Firecrawl]
  SV --> N[Nexar]
  SV --> L[LCSC]
```

| Layer | Path | Responsibility |
|-------|------|----------------|
| Router | `src/router.rs` | Routes, `AppState` |
| Handlers | `src/handlers/` | HTTP adapters, `AppError` mapping |
| Store | `src/store/` | Queries and transactions |
| Services | `src/services/` | AI build, validation, exports, catalog |

Key services: `design_pipeline`, `schematic_gen`, `schematic_validate`, `erc_fixes`, `sexp_netlist`, `svg_export`, `pdf_export`, `catalog_search`, `lcsc`, `nexar`.

## AI build data flow

```mermaid
flowchart TD
  P[User prompt] --> I[(design_intents)]
  I --> X1[xAI<br/>queries · candidate MPNs]
  X1 --> FC[Firecrawl]
  FC --> RA[(design_research_artifacts)]
  RA --> X2[xAI<br/>resolve BOM]
  X2 --> C[(manufacturers · parts)]
  X2 --> B[(bom_lines)]
  C --> X3[xAI<br/>ReplaceSchematic]
  B --> X3
  X3 --> R[Schematic JSON<br/>BOM-grounded part_id only]
```

## Data model

| Area | Tables |
|------|--------|
| Identity | users, API keys, usage |
| Catalog | manufacturers, parts, part_offers |
| Design | designs, design_intents, design_research_artifacts |
| Schematic | schematic_instances, schematic_nets, schematic_pins; `schematic_document` JSONB |
| BOM | bom_lines |

Domain relationships (simplified):

```mermaid
flowchart LR
  subgraph id[Identity]
    U[users]
    A[API keys]
  end
  subgraph cat[Catalog]
    MF[manufacturers]
    PT[parts]
    PO[part_offers]
  end
  subgraph des[Design]
    D[designs]
    IN[design_intents]
    RE[design_research_artifacts]
  end
  subgraph sch[Schematic]
    SI["schematic_* tables"]
    DOC[schematic_document JSONB]
  end
  subgraph bom[BOM]
    BL[bom_lines]
  end
  U --> D
  MF --> PT
  PT --> PO
  D --> IN
  D --> RE
  D --> SI
  D --> DOC
  D --> BL
  PT --> BL
```

## Native editor

| Module | Role |
|--------|------|
| `native/src/editor/` | Canvas, tools, hit-test, render, undo |
| `native/src/app/studio/` | Dock UI, place panel, inspector, Build tab |
| `native/src/base_symbols/` | Load `.tokito_sym` from `assets/base-symbols/` + user dir |
| `native/src/symbol_format/` | Tokito symbol S-expression parser |

Native save path:

```mermaid
flowchart LR
  G[Canvas graph] --> SD[SchematicDocument]
  SD --> E[ERC]
  E --> PG[(Postgres<br/>schematic + document JSON)]
```

## Operations

- **Database:** pg-embed under the user data dir (`TOKITO_EMBEDDED_PORT`, default `15432`). Hold the `EmbeddedPostgres` handle for the process lifetime.
- **Pool size:** `TOKITO_DB_MAX_CONNECTIONS` (default 10).
- **Migrations:** applied at process start.
- **Secrets:** set `TOKITO_JWT_SECRET` in release builds; never commit `.env`.

## Related

- [API.md](API.md) — route reference

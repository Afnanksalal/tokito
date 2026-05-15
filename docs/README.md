# Documentation

Technical documentation for **Tokito**—the API, system shape, and product roadmap. Start with the repository **[README](../README.md)** for setup, environment variables, and the **copilot pipeline** overview.

---

## Guides

| Document | Audience | What it covers |
|----------|----------|----------------|
| [**API.md**](API.md) | Integrators, frontend authors | REST routes under `/v1`, JSON bodies, errors, copilot endpoints |
| [**ARCHITECTURE.md**](ARCHITECTURE.md) | Contributors, operators | Layers (HTTP → handlers → services → store), Postgres model, migrations |
| [**PRODUCT_PLAN.md**](PRODUCT_PLAN.md) | Product & engineering | Staged roadmap (intent → research → grounded schematic → canvas → ERC → exports), implementation status |
| [**EDITOR_PLAN.md**](EDITOR_PLAN.md) | Product & engineering | Production schematic editor direction: data model, CAD UI, canvas tools, ERC, and implementation milestones |

---

## Conventions

- **Base URL** examples assume `http://localhost:8080`; replace with your deployment host and TLS as needed.
- **JSON** bodies use `Content-Type: application/json` unless noted.
- **Authentication**: `/v1/*` routes expect a valid JWT (see API login/register flows). The **native app** uses a local bootstrap user for single-seat workflows.
- **Versioning**: API paths are prefixed with **`/v1`**.

When behavior or env vars change, update **API.md** and the root **README** so they stay aligned.

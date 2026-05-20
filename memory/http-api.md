# HTTP API surface (optional)

The HTTP layer (`src/router.rs`, handlers under `src/handlers/`) is **explicitly secondary**: `docs/API.md` says end users run `Tokito.exe` and don't call HTTP. The HTTP binary is used for automated tests + non-default deployments.

Shape: `GET /health` + `/v1/*`. `/v1/auth/register` and `/v1/auth/login` are public; `/v1/auth/api-keys` is nested under auth but **JWT-protected**. The rest of `/v1` is also JWT-protected via `auth::middleware::require_auth`. Routes cover manufacturers, parts, designs, intent, research (scrape/search/notes/annotate), BOM (get/put/append), schematic graph (`/schematic`, `/schematic/document`, `/schematic/validate`, `/schematic/suggest` — the AI build entrypoint), agent runs (`POST /v1/agent/run`), and integration proxies (`/v1/integrations/{firecrawl,ai,xai}/...` — `xai` path is retained as a back-compat alias for `ai`).

When `TOKITO_STATIC_DIR` is set, the router can fall back to a static SPA build, but the main binary defaults to no UI. CORS origins come from config.

**How to apply:** when changing schemas exposed over `/v1`, also update `docs/API.md` per CONTRIBUTING. Don't pitch new features as web-API-first; the canonical surface is the native studio.

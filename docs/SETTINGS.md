# Settings reference

Primary configuration: **`%LOCALAPPDATA%\tokito\settings.toml`** (Studio > **Settings**).

On first launch, Tokito may import a legacy **`.env`** file beside the executable (or in the working directory) into `settings.toml` once.

## Always on (built-in)

These are not toggles; they are part of the product defaults:

- **OS keychain** for API keys (AI provider, Firecrawl, Nexar)
- **Firecrawl incremental build** (skips cached research when possible)
- **ERC strict** (blocks export when ERC errors remain)
- **Bus tool** in the schematic toolbar
- **LCSC catalog search**
- **Auto-add** placed catalog parts to the BOM
- **Open + reveal in folder** after export

## General (user preferences)

| Key | Description |
|-----|-------------|
| `theme` | `light`, `dark`, or `system` (default `system`) |
| `default_export_format` | `pdf`, `svg`, or `bundle` |

## Database

| Key | Description |
|-----|-------------|
| `embedded_port` | Embedded Postgres port (default `15432`) |
| `pg_embed_version` | `16`, `17`, or `18` |
| `max_connections` | Connection pool size |
| `data_dir` | Custom cluster path (empty = default) |

## AI (required for Build)

| Key | Description |
|-----|-------------|
| `provider` | `xai`, `openai`, `anthropic`, `gemini`, or `kimi` |
| `llm_api_key` | API key for the selected provider |
| `llm_base_url` | Optional provider base URL override |
| `firecrawl_api_key` | Firecrawl API key |
| `firecrawl_base_url` | Firecrawl base URL |
| `agent_model` | LLM model id |
| `agent_max_iterations` | Agent loop limit |
| `agent_max_llm_tokens` | Token budget per run |

Legacy `xai_api_key`, `xai_base_url`, `TOKITO_XAI_API_KEY`, and `TOKITO_XAI_BASE_URL`
are still accepted as compatibility aliases.

## Catalog

| Key | Description |
|-----|-------------|
| `nexar_client_id` / `nexar_client_secret` | Optional Nexar OAuth for richer distributor metadata |

## Server (advanced)

Used by the `tokito` HTTP binary only:

| Key | Description |
|-----|-------------|
| `http_addr` | Bind address |
| `jwt_secret` | JWT signing secret |

## Process environment (CI only)

Empty fields in `settings.toml` can be filled from `TOKITO_*` variables. This does not disable built-in features.

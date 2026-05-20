# Env vars (reference)

Env vars are an **overlay**, not the source of truth — `settings.toml` is. `merge_from_env` (in `src/settings.rs`) only fills *empty* fields. Setting any of the AI-related runtime vars also flips `general.settings_migrated_from_env = true`.

**Runtime / settings overlay** (`src/settings.rs::merge_from_env`):

| Var | Fills | Notes |
|---|---|---|
| `TOKITO_AI_PROVIDER` | `ai.provider` | One of `openai`/`anthropic`/`gemini`/`xai`/`kimi`; unknown → `xai`. |
| `TOKITO_LLM_API_KEY` | `ai.llm_api_key` | Alias: **`TOKITO_XAI_API_KEY`** (legacy, still honored via `or_else`). |
| `TOKITO_LLM_BASE_URL` | `ai.llm_base_url` | Alias: **`TOKITO_XAI_BASE_URL`** (legacy). |
| `TOKITO_FIRECRAWL_API_KEY` | `ai.firecrawl_api_key` | Required for the Build/research pipeline. |
| `TOKITO_EMBEDDED_PORT` | `database.embedded_port` | Parsed as `u16`; silently dropped if non-numeric. |
| `TOKITO_PG_EMBED_VERSION` | `database.pg_embed_version` | Parsed as `u16` (valid: `16`/`17`/`18`). |
| `TOKITO_LCSC_ANONYMOUS_SEARCH` | `catalog.lcsc_anonymous_search` | Truthy: `1`/`true`/`yes` (case-insensitive). Other values leave the field as-is. |
| `TOKITO_NEXAR_CLIENT_ID` | `catalog.nexar_client_id` | |
| `TOKITO_NEXAR_CLIENT_SECRET` | `catalog.nexar_client_secret` | |

`src/config_provider.rs` does **not** consume/remove runtime env vars in production. The `env::remove_var("TOKITO_XAI_API_KEY")` call there is test cleanup only.

**One-time legacy `.env` import** (`src/settings.rs::import_legacy_dotenv_files`):

This reads `.env` beside the executable or in the current working directory and persists recognized keys into `settings.toml` once. It accepts the runtime overlay vars above, plus:

| Var | Fills | Notes |
|---|---|---|
| `TOKITO_FIRECRAWL_BASE_URL` | `ai.firecrawl_base_url` | Legacy `.env` only; not a live process overlay. |
| `TOKITO_DB_MAX_CONNECTIONS` | `database.max_connections` | Parsed as `u32`; legacy `.env` only. |
| `TOKITO_NEXAR_SCOPE` | `catalog.nexar_scope` | Legacy `.env` only. |
| `TOKITO_AGENT_MODEL` | `ai.agent_model` | Legacy `.env` only. |
| `TOKITO_AGENT_MAX_ITERATIONS` | `ai.agent_max_iterations` | Parsed as `u32`; legacy `.env` only. |
| `TOKITO_AGENT_MAX_LLM_TOKENS` | `ai.agent_max_llm_tokens` | Parsed as `i64`; legacy `.env` only. |
| `TOKITO_JWT_SECRET` | `server.jwt_secret` | Legacy `.env` only; `settings.toml` auto-generates this when missing. |
| `TOKITO_HTTP_ADDR` | `server.http_addr` | Legacy `.env` only; default is `127.0.0.1:8080`. |

**HTTP binary** (`src/main.rs`):

| Var | Effect |
|---|---|
| `TOKITO_STATIC_DIR` | If set + dir + `index.html`, the Axum router serves an SPA fallback for non-`/v1` GETs. Trimmed; empty string treated as unset. |
| `RUST_LOG` | Standard `tracing_subscriber` env-filter; defaults to `tokito=info,tower_http=info` if unset. |

**Test harness** (`src/test_support.rs`):

| Var | Effect |
|---|---|
| **`TOKITO_RUN_DB_INTEGRATION=1`** | Required to run the embedded-Postgres integration suite. This is the **only** switch — the old `GITHUB_ACTIONS=true` auto-enable was removed; CI sets the var explicitly. |
| `TOKITO_TEST_EMBEDDED_PORT` | Override the port the test cluster binds to. |

**Secrets are NOT env vars.** Production keys (LLM, Firecrawl, Nexar) belong in the **OS keychain** via `src/secrets.rs` + the `keyring` crate. Env vars are for CI / dev shells / one-shot bootstrap; reading them at runtime in new code is a smell.

**What is *not* a live process env overlay:** `http_addr`, `jwt_secret`, `cors_origins`, agent limits, theme, ERC strict, bus tool, BOM auto-add, export open/reveal — these are read from `settings.toml` (`SettingsFile`) and the derived `Config`. `TOKITO_HTTP_ADDR` and `TOKITO_JWT_SECRET` exist only for the one-time legacy `.env` import path.

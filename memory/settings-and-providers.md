# Settings & AI providers

**Settings file** is the primary config, not `.env`:

- Path: **`%LOCALAPPDATA%\tokito\settings.toml`** on Windows (the OS app-data dir on other platforms).
- A one-time legacy `.env` import is supported (`settings_migrated_from_env` flag in `GeneralSettings`).
- `TOKITO_*` env vars **only fill empty fields** in `settings.toml`; they don't disable built-ins. Built-ins (always on): OS keychain, Firecrawl incremental build, ERC strict, bus tool, LCSC catalog, BOM auto-add, open/reveal after export.

**AI providers** (`src/config.rs::AiProvider`): `OpenAi`, `Anthropic`, `Gemini`, `Xai`, `Kimi`. `parse()` defaults to `Xai` for unknown strings. Default models hardcoded in `default_model()` — note **these are forward-looking IDs** (`gpt-5.5`, `claude-sonnet-4-5`, `grok-4.3`, `gemini-2.5-flash`, `kimi-k2.6`); verify against the file before quoting, they may have drifted.

**Why:** the provider commit `7a89b67 Replace xAI with generic AI provider, bump deps` genericized the AI layer — older docs still mention xAI specifically and legacy `xai_*` / `TOKITO_XAI_*` keys remain as compatibility aliases. Don't reintroduce xAI-specific assumptions when editing AI code.

**How to apply:** when adding settings, document them in `docs/SETTINGS.md` (CONTRIBUTING.md mandates this) and update the `SettingsFile` structs in `src/settings.rs`. Secrets go through `src/secrets.rs` / OS keychain — never read them from `settings.toml` directly in new code.

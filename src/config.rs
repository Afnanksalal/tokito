//! Environment configuration for the API process.

use std::env;

/// xAI (Grok) — OpenAI-compatible API. See [xAI docs](https://docs.x.ai/docs/tutorial).
#[derive(Debug, Clone)]
pub struct XaiConfig {
    pub api_key: String,
    pub base_url: String,
}

/// Firecrawl API host (default `https://api.firecrawl.dev/v1` for scrape).
/// Search requests use `{root}/v2/search` where `root` strips a trailing `/v1` or `/v2`.
#[derive(Debug, Clone)]
pub struct FirecrawlConfig {
    pub api_key: String,
    pub base_url: String,
}

/// Nexar Supply (Octopart successor) OAuth client credentials.
#[derive(Debug, Clone)]
pub struct NexarConfig {
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
}

#[derive(Debug, Clone)]
pub struct AgentLimits {
    pub max_iterations: u32,
    pub max_llm_tokens_per_run: i64,
    pub default_model: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub http_addr: String,
    pub database_url: String,
    pub db_max_connections: u32,
    /// Allowed browser origins for CORS; empty slice means mirror request (dev only).
    pub cors_origins: Vec<String>,
    pub jwt_secret: String,
    pub xai: Option<XaiConfig>,
    pub firecrawl: Option<FirecrawlConfig>,
    pub nexar: Option<NexarConfig>,
    pub lcsc_anonymous_search: bool,
    pub agent: AgentLimits,
}

/// Loads configuration from `TOKITO_*` environment variables.
pub fn load() -> anyhow::Result<Config> {
    let http_addr = env::var("TOKITO_HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let database_url = env::var("TOKITO_DATABASE_URL").map_err(|_| {
        anyhow::anyhow!("TOKITO_DATABASE_URL is required (PostgreSQL connection string)")
    })?;
    let db_max_connections: u32 = env::var("TOKITO_DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let cors_origins = env::var("TOKITO_CORS_ORIGINS")
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let jwt_secret = env::var("TOKITO_JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("TOKITO_JWT_SECRET not set; using insecure development default");
        "tokito-dev-insecure-jwt-secret-change-me".to_string()
    });

    let xai = env::var("TOKITO_XAI_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|api_key| XaiConfig {
            api_key,
            base_url: env::var("TOKITO_XAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.x.ai/v1".to_string()),
        });

    let firecrawl = env::var("TOKITO_FIRECRAWL_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|api_key| FirecrawlConfig {
            api_key,
            base_url: env::var("TOKITO_FIRECRAWL_BASE_URL")
                .unwrap_or_else(|_| "https://api.firecrawl.dev/v1".to_string()),
        });

    let nexar = match (
        env::var("TOKITO_NEXAR_CLIENT_ID").ok(),
        env::var("TOKITO_NEXAR_CLIENT_SECRET").ok(),
    ) {
        (Some(cid), Some(sec)) if !cid.trim().is_empty() && !sec.trim().is_empty() => {
            Some(NexarConfig {
                client_id: cid,
                client_secret: sec,
                scope: env::var("TOKITO_NEXAR_SCOPE")
                    .unwrap_or_else(|_| "nexar-supply".to_string()),
            })
        }
        _ => None,
    };

    let lcsc_anonymous_search = env::var("TOKITO_LCSC_ANONYMOUS_SEARCH")
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "0" | "false" | "no" => Some(false),
            "1" | "true" | "yes" => Some(true),
            _ => None,
        })
        .unwrap_or(true);

    let agent = AgentLimits {
        max_iterations: env::var("TOKITO_AGENT_MAX_ITERATIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10),
        max_llm_tokens_per_run: env::var("TOKITO_AGENT_MAX_LLM_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(96_000),
        default_model: env::var("TOKITO_AGENT_MODEL").unwrap_or_else(|_| "grok-4.3".to_string()),
    };

    Ok(Config {
        http_addr,
        database_url,
        db_max_connections,
        cors_origins,
        jwt_secret,
        xai,
        firecrawl,
        nexar,
        lcsc_anonymous_search,
        agent,
    })
}

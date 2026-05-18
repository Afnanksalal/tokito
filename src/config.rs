//! Application configuration (`settings.toml` via [`crate::config_provider`]).

/// xAI Grok (OpenAI-compatible chat API).
#[derive(Debug, Clone)]
pub struct XaiConfig {
    pub api_key: String,
    pub base_url: String,
}

/// Firecrawl API host (default `https://api.firecrawl.dev/v1` for scrape).
#[derive(Debug, Clone)]
pub struct FirecrawlConfig {
    pub api_key: String,
    pub base_url: String,
}

/// Nexar Supply OAuth client credentials.
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
    pub db_max_connections: u32,
    pub embedded_port: u16,
    pub pg_embed_version: u16,
    pub cors_origins: Vec<String>,
    pub jwt_secret: String,
    pub xai: Option<XaiConfig>,
    pub firecrawl: Option<FirecrawlConfig>,
    pub nexar: Option<NexarConfig>,
    pub lcsc_anonymous_search: bool,
    pub agent: AgentLimits,
}

/// Loads configuration from `settings.toml` (merged with env for empty keys).
pub fn load() -> anyhow::Result<Config> {
    crate::config_provider::load_config()
}

/// Loads configuration via an explicit provider (tests / HTTP service).
pub fn load_from_provider(provider: &dyn crate::config_provider::ConfigProvider) -> anyhow::Result<Config> {
    provider.load_config()
}

#[cfg(feature = "test-support")]
impl Config {
    pub fn for_tests() -> Self {
        Self {
            http_addr: "127.0.0.1:0".into(),
            db_max_connections: 5,
            embedded_port: 17_334,
            pg_embed_version: 16,
            cors_origins: vec![],
            jwt_secret: "test-jwt-secret".into(),
            xai: None,
            firecrawl: None,
            nexar: None,
            lcsc_anonymous_search: true,
            agent: AgentLimits {
                max_iterations: 5,
                max_llm_tokens_per_run: 8000,
                default_model: "test".into(),
            },
        }
    }
}

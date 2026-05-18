//! Application configuration (`settings.toml` via [`crate::config_provider`]).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProvider {
    OpenAi,
    Anthropic,
    Gemini,
    Xai,
    Kimi,
}

impl AiProvider {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" | "open_ai" => Self::OpenAi,
            "anthropic" | "claude" => Self::Anthropic,
            "gemini" | "google" | "google-gemini" => Self::Gemini,
            "kimi" | "moonshot" => Self::Kimi,
            _ => Self::Xai,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Xai => "xai",
            Self::Kimi => "kimi",
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::Xai => "https://api.x.ai/v1",
            Self::Kimi => "https://api.moonshot.ai/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-5.5",
            Self::Anthropic => "claude-sonnet-4-5",
            Self::Gemini => "gemini-2.5-flash",
            Self::Xai => "grok-4.3",
            Self::Kimi => "kimi-k2.6",
        }
    }
}

/// Configured AI chat provider for Build and Agent.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: AiProvider,
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
    pub postgres_data_dir: std::path::PathBuf,
    pub cors_origins: Vec<String>,
    pub jwt_secret: String,
    pub llm: Option<LlmConfig>,
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
pub fn load_from_provider(
    provider: &dyn crate::config_provider::ConfigProvider,
) -> anyhow::Result<Config> {
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
            postgres_data_dir: crate::paths::default_postgres_data_dir(),
            cors_origins: vec![],
            jwt_secret: "test-jwt-secret".into(),
            llm: None,
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

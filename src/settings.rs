//! Persistent user settings (`settings.toml` under app data).

use crate::config::{AgentLimits, Config, FirecrawlConfig, NexarConfig, XaiConfig};
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsFile {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub database: DatabaseSettings,
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub catalog: CatalogSettings,
    #[serde(default)]
    pub server: ServerSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default = "default_true")]
    pub export_open_after_save: bool,
    #[serde(default = "default_true")]
    pub export_reveal_in_folder: bool,
    #[serde(default = "default_true")]
    pub use_keychain: bool,
    #[serde(default = "default_export_format")]
    pub default_export_format: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_true")]
    pub erc_strict_mode: bool,
    #[serde(default = "default_true")]
    pub enable_bus_tool: bool,
    #[serde(default = "default_true")]
    pub auto_add_placed_parts_to_bom: bool,
    #[serde(default)]
    pub settings_migrated_from_env: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            export_open_after_save: true,
            export_reveal_in_folder: true,
            use_keychain: true,
            default_export_format: default_export_format(),
            theme: default_theme(),
            erc_strict_mode: true,
            enable_bus_tool: true,
            auto_add_placed_parts_to_bom: true,
            settings_migrated_from_env: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_embedded_port")]
    pub embedded_port: u16,
    #[serde(default = "default_pg_version")]
    pub pg_embed_version: u16,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default)]
    pub data_dir: String,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            embedded_port: default_embedded_port(),
            pg_embed_version: default_pg_version(),
            max_connections: default_max_connections(),
            data_dir: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default)]
    pub xai_api_key: String,
    #[serde(default = "default_xai_base")]
    pub xai_base_url: String,
    #[serde(default)]
    pub firecrawl_api_key: String,
    #[serde(default = "default_firecrawl_base")]
    pub firecrawl_base_url: String,
    #[serde(default = "default_agent_model")]
    pub agent_model: String,
    #[serde(default = "default_agent_iterations")]
    pub agent_max_iterations: u32,
    #[serde(default = "default_agent_tokens")]
    pub agent_max_llm_tokens: i64,
    #[serde(default = "default_true")]
    pub incremental_build: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            xai_api_key: String::new(),
            xai_base_url: default_xai_base(),
            firecrawl_api_key: String::new(),
            firecrawl_base_url: default_firecrawl_base(),
            agent_model: default_agent_model(),
            agent_max_iterations: default_agent_iterations(),
            agent_max_llm_tokens: default_agent_tokens(),
            incremental_build: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSettings {
    #[serde(default = "default_true")]
    pub lcsc_anonymous_search: bool,
    #[serde(default)]
    pub nexar_client_id: String,
    #[serde(default)]
    pub nexar_client_secret: String,
    #[serde(default = "default_nexar_scope")]
    pub nexar_scope: String,
}

impl Default for CatalogSettings {
    fn default() -> Self {
        Self {
            lcsc_anonymous_search: true,
            nexar_client_id: String::new(),
            nexar_client_secret: String::new(),
            nexar_scope: default_nexar_scope(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_http_addr")]
    pub http_addr: String,
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            http_addr: default_http_addr(),
            jwt_secret: String::new(),
            cors_origins: vec![],
        }
    }
}

impl Default for SettingsFile {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            database: DatabaseSettings::default(),
            ai: AiSettings::default(),
            catalog: CatalogSettings::default(),
            server: ServerSettings::default(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "system".into()
}

/// Core product behavior is always on (not user-disableable).
pub fn apply_product_defaults(settings: &mut SettingsFile) {
    settings.general.export_open_after_save = true;
    settings.general.export_reveal_in_folder = true;
    settings.general.use_keychain = true;
    settings.general.erc_strict_mode = true;
    settings.general.enable_bus_tool = true;
    settings.general.auto_add_placed_parts_to_bom = true;
    settings.ai.incremental_build = true;
    settings.catalog.lcsc_anonymous_search = true;
    if settings.general.theme.trim().is_empty() {
        settings.general.theme = default_theme();
    }
    if settings.ai.xai_base_url.trim().is_empty() {
        settings.ai.xai_base_url = default_xai_base();
    }
    if settings.ai.firecrawl_base_url.trim().is_empty() {
        settings.ai.firecrawl_base_url = default_firecrawl_base();
    }
}
fn default_embedded_port() -> u16 {
    15_432
}
fn default_pg_version() -> u16 {
    16
}
fn default_max_connections() -> u32 {
    10
}
fn default_xai_base() -> String {
    "https://api.x.ai/v1".into()
}
fn default_firecrawl_base() -> String {
    "https://api.firecrawl.dev/v1".into()
}
fn default_agent_model() -> String {
    "grok-4.3".into()
}
fn default_agent_iterations() -> u32 {
    10
}
fn default_agent_tokens() -> i64 {
    96_000
}
fn default_nexar_scope() -> String {
    "nexar-supply".into()
}
fn default_http_addr() -> String {
    "0.0.0.0:8080".into()
}

pub fn load_file() -> SettingsFile {
    let path = paths::settings_path();
    let mut s = if !path.is_file() {
        SettingsFile::default()
    } else {
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(%e, "invalid settings.toml — using defaults");
                SettingsFile::default()
            }),
            Err(e) => {
                tracing::warn!(%e, "could not read settings.toml");
                SettingsFile::default()
            }
        }
    };
    apply_product_defaults(&mut s);
    if !path.is_file() {
        let _ = save_file(&s);
    }
    s
}

pub fn save_file(settings: &SettingsFile) -> anyhow::Result<()> {
    let path = paths::settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(settings)?;
    fs::write(path, text)?;
    Ok(())
}

fn default_export_format() -> String {
    "pdf".into()
}

/// One-time import from legacy `.env` files (no `dotenvy` dependency).
pub fn import_legacy_dotenv_files(settings: &mut SettingsFile) -> bool {
    let mut changed = false;
    let mut paths = vec![crate::paths::exe_dir().join(".env")];
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(".env");
        if !paths.iter().any(|x| x == &p) {
            paths.push(p);
        }
    }
    for path in paths {
        if !path.as_path().is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (key, value) in parse_dotenv_lines(&text) {
            if apply_legacy_env_pair(settings, &key, &value) {
                changed = true;
            }
        }
    }
    changed
}

fn parse_dotenv_lines(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim().to_string();
        let mut value = v.trim().to_string();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = value[1..value.len().saturating_sub(1)].to_string();
        }
        if !key.is_empty() {
            out.push((key, value));
        }
    }
    out
}

fn apply_legacy_env_pair(settings: &mut SettingsFile, key: &str, value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    match key {
        "TOKITO_XAI_API_KEY" if settings.ai.xai_api_key.is_empty() => {
            settings.ai.xai_api_key = value.into();
            true
        }
        "TOKITO_FIRECRAWL_API_KEY" if settings.ai.firecrawl_api_key.is_empty() => {
            settings.ai.firecrawl_api_key = value.into();
            true
        }
        "TOKITO_XAI_BASE_URL" if settings.ai.xai_base_url.is_empty() => {
            settings.ai.xai_base_url = value.into();
            true
        }
        "TOKITO_FIRECRAWL_BASE_URL" if settings.ai.firecrawl_base_url.is_empty() => {
            settings.ai.firecrawl_base_url = value.into();
            true
        }
        "TOKITO_EMBEDDED_PORT" => value.parse::<u16>().ok().map(|p| {
            settings.database.embedded_port = p;
            true
        }).unwrap_or(false),
        "TOKITO_PG_EMBED_VERSION" => value.parse::<u16>().ok().map(|p| {
            settings.database.pg_embed_version = p;
            true
        }).unwrap_or(false),
        "TOKITO_DB_MAX_CONNECTIONS" => value.parse::<u32>().ok().map(|p| {
            settings.database.max_connections = p;
            true
        }).unwrap_or(false),
        "TOKITO_LCSC_ANONYMOUS_SEARCH" => {
            if matches!(value.to_lowercase().as_str(), "1" | "true" | "yes") {
                settings.catalog.lcsc_anonymous_search = true;
            }
            true
        }
        "TOKITO_NEXAR_CLIENT_ID" if settings.catalog.nexar_client_id.is_empty() => {
            settings.catalog.nexar_client_id = value.into();
            true
        }
        "TOKITO_NEXAR_CLIENT_SECRET" if settings.catalog.nexar_client_secret.is_empty() => {
            settings.catalog.nexar_client_secret = value.into();
            true
        }
        "TOKITO_NEXAR_SCOPE" if settings.catalog.nexar_scope.is_empty() => {
            settings.catalog.nexar_scope = value.into();
            true
        }
        "TOKITO_AGENT_MODEL" if settings.ai.agent_model.is_empty() => {
            settings.ai.agent_model = value.into();
            true
        }
        "TOKITO_AGENT_MAX_ITERATIONS" => value.parse::<u32>().ok().map(|p| {
            settings.ai.agent_max_iterations = p;
            true
        }).unwrap_or(false),
        "TOKITO_AGENT_MAX_LLM_TOKENS" => value.parse::<i64>().ok().map(|p| {
            settings.ai.agent_max_llm_tokens = p;
            true
        }).unwrap_or(false),
        "TOKITO_JWT_SECRET" if settings.server.jwt_secret.is_empty() => {
            settings.server.jwt_secret = value.into();
            true
        }
        "TOKITO_HTTP_ADDR" if settings.server.http_addr.is_empty() => {
            settings.server.http_addr = value.into();
            true
        }
        _ => false,
    }
}

/// Overlay empty settings fields from process environment (CI / dev shells).
pub fn merge_from_env(mut s: SettingsFile) -> SettingsFile {
    use std::env;
    let mut imported = false;
    if s.ai.xai_api_key.is_empty() {
        if let Ok(v) = env::var("TOKITO_XAI_API_KEY") {
            if !v.is_empty() {
                s.ai.xai_api_key = v;
                imported = true;
            }
        }
    }
    if s.ai.firecrawl_api_key.is_empty() {
        if let Ok(v) = env::var("TOKITO_FIRECRAWL_API_KEY") {
            if !v.is_empty() {
                s.ai.firecrawl_api_key = v;
                imported = true;
            }
        }
    }
    if let Ok(v) = env::var("TOKITO_EMBEDDED_PORT") {
        if let Ok(p) = v.parse() {
            s.database.embedded_port = p;
        }
    }
    if let Ok(v) = env::var("TOKITO_PG_EMBED_VERSION") {
        if let Ok(p) = v.parse() {
            s.database.pg_embed_version = p;
        }
    }
    if let Ok(v) = env::var("TOKITO_LCSC_ANONYMOUS_SEARCH") {
        if matches!(v.to_lowercase().as_str(), "1" | "true" | "yes") {
            s.catalog.lcsc_anonymous_search = true;
        }
    }
    if s.catalog.nexar_client_id.is_empty() {
        if let Ok(v) = env::var("TOKITO_NEXAR_CLIENT_ID") {
            s.catalog.nexar_client_id = v;
        }
    }
    if s.catalog.nexar_client_secret.is_empty() {
        if let Ok(v) = env::var("TOKITO_NEXAR_CLIENT_SECRET") {
            s.catalog.nexar_client_secret = v;
        }
    }
    if imported {
        s.general.settings_migrated_from_env = true;
    }
    apply_product_defaults(&mut s);
    s
}

pub fn export_redacted(settings: &SettingsFile) -> String {
    let mut copy = settings.clone();
    copy.ai.xai_api_key = redact(&copy.ai.xai_api_key);
    copy.ai.firecrawl_api_key = redact(&copy.ai.firecrawl_api_key);
    copy.catalog.nexar_client_secret = redact(&copy.catalog.nexar_client_secret);
    copy.server.jwt_secret = redact(&copy.server.jwt_secret);
    toml::to_string_pretty(&copy).unwrap_or_default()
}

fn redact(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        "***".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_toml() {
        let s = SettingsFile::default();
        let text = toml::to_string_pretty(&s).unwrap();
        let back: SettingsFile = toml::from_str(&text).unwrap();
        assert_eq!(back.general.theme, s.general.theme);
        assert_eq!(back.database.embedded_port, s.database.embedded_port);
    }

    #[test]
    fn apply_product_defaults_enables_core_features() {
        let mut s = SettingsFile::default();
        s.general.erc_strict_mode = false;
        s.general.use_keychain = false;
        s.catalog.lcsc_anonymous_search = false;
        apply_product_defaults(&mut s);
        assert!(s.general.erc_strict_mode);
        assert!(s.general.use_keychain);
        assert!(s.catalog.lcsc_anonymous_search);
        assert!(s.ai.incremental_build);
    }

    #[test]
    fn import_legacy_dotenv_parses_keys() {
        let mut s = SettingsFile::default();
        let text = "TOKITO_XAI_API_KEY=from-file\nTOKITO_FIRECRAWL_API_KEY=fc\n";
        for (k, v) in parse_dotenv_lines(text) {
            apply_legacy_env_pair(&mut s, &k, &v);
        }
        assert_eq!(s.ai.xai_api_key, "from-file");
        assert_eq!(s.ai.firecrawl_api_key, "fc");
    }
}

pub fn postgres_data_dir(settings: &SettingsFile) -> std::path::PathBuf {
    if settings.database.data_dir.trim().is_empty() {
        paths::default_postgres_data_dir()
    } else {
        std::path::PathBuf::from(settings.database.data_dir.trim())
    }
}

impl SettingsFile {
    pub fn to_config(&self) -> anyhow::Result<Config> {
        let jwt_secret = if self.server.jwt_secret.trim().is_empty() {
            if cfg!(debug_assertions) {
                "tokito-dev-insecure-jwt-secret-change-me".to_string()
            } else {
                anyhow::bail!("jwt_secret is required in settings (server.jwt_secret)");
            }
        } else {
            self.server.jwt_secret.clone()
        };

        let xai = if self.ai.xai_api_key.trim().is_empty() {
            None
        } else {
            Some(XaiConfig {
                api_key: self.ai.xai_api_key.trim().to_string(),
                base_url: self.ai.xai_base_url.clone(),
            })
        };

        let firecrawl = if self.ai.firecrawl_api_key.trim().is_empty() {
            None
        } else {
            Some(FirecrawlConfig {
                api_key: self.ai.firecrawl_api_key.trim().to_string(),
                base_url: self.ai.firecrawl_base_url.clone(),
            })
        };

        let nexar = if self.catalog.nexar_client_id.trim().is_empty()
            || self.catalog.nexar_client_secret.trim().is_empty()
        {
            None
        } else {
            Some(NexarConfig {
                client_id: self.catalog.nexar_client_id.trim().to_string(),
                client_secret: self.catalog.nexar_client_secret.trim().to_string(),
                scope: self.catalog.nexar_scope.clone(),
            })
        };

        Ok(Config {
            http_addr: self.server.http_addr.clone(),
            db_max_connections: self.database.max_connections,
            embedded_port: self.database.embedded_port,
            cors_origins: self.server.cors_origins.clone(),
            jwt_secret,
            xai,
            firecrawl,
            nexar,
            lcsc_anonymous_search: true,
            agent: AgentLimits {
                max_iterations: self.ai.agent_max_iterations,
                max_llm_tokens_per_run: self.ai.agent_max_llm_tokens,
                default_model: self.ai.agent_model.clone(),
            },
            pg_embed_version: self.database.pg_embed_version,
        })
    }
}

pub fn load_merged_config() -> anyhow::Result<Config> {
    crate::config_provider::load_config()
}

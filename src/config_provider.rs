//! Layered configuration: defaults, settings.toml, env overlay (CI), keychain.

use crate::config::Config;
use crate::settings::{load_file, merge_from_env, SettingsFile};

/// Loads persisted settings and runtime `Config`.
pub trait ConfigProvider: Send + Sync {
    fn load_settings(&self) -> SettingsFile;
    fn load_config(&self) -> anyhow::Result<Config> {
        self.load_settings().to_config()
    }
}

/// Reads `{app_data}/tokito/settings.toml`.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileSettingsProvider;

impl ConfigProvider for FileSettingsProvider {
    fn load_settings(&self) -> SettingsFile {
        let mut s = load_file();
        if crate::settings::import_legacy_dotenv_files(&mut s) {
            s.general.settings_migrated_from_env = true;
        }
        crate::settings::apply_product_defaults(&mut s);
        if s.general.settings_migrated_from_env {
            let _ = crate::settings::save_file(&s);
        }
        s
    }
}

/// Applies `TOKITO_*` env for keys still empty in file settings (CI / legacy).
#[derive(Debug, Clone)]
pub struct EnvOverlayProvider<P> {
    pub inner: P,
}

impl<P: ConfigProvider> ConfigProvider for EnvOverlayProvider<P> {
    fn load_settings(&self) -> SettingsFile {
        let mut s = merge_from_env(self.inner.load_settings());
        crate::secrets::apply_keychain_to_settings(&mut s);
        crate::settings::apply_product_defaults(&mut s);
        s
    }
}

/// Default stack: file + env overlay.
pub fn default_provider() -> EnvOverlayProvider<FileSettingsProvider> {
    EnvOverlayProvider {
        inner: FileSettingsProvider,
    }
}

pub fn load_config() -> anyhow::Result<Config> {
    default_provider().load_config()
}

pub fn load_settings_merged() -> SettingsFile {
    default_provider().load_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_provider_returns_defaults_when_missing() {
        let s = FileSettingsProvider.load_settings();
        assert!(!s.general.theme.is_empty());
    }

    #[test]
    fn env_overlay_merges_legacy_xai_key() {
        struct EmptyProvider;
        impl ConfigProvider for EmptyProvider {
            fn load_settings(&self) -> crate::settings::SettingsFile {
                crate::settings::SettingsFile::default()
            }
        }
        std::env::set_var("TOKITO_XAI_API_KEY", "overlay-test");
        let s = EnvOverlayProvider {
            inner: EmptyProvider,
        }
        .load_settings();
        assert_eq!(s.ai.llm_api_key, "overlay-test");
        std::env::remove_var("TOKITO_XAI_API_KEY");
    }
}

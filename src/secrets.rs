//! OS keychain storage for API secrets (always used when the platform supports it).

const SERVICE: &str = "tokito";

pub fn get_secret(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key).ok()?.get_password().ok()
}

pub fn set_secret(key: &str, value: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(SERVICE, key)?;
    if value.is_empty() {
        let _ = entry.delete_credential();
        Ok(())
    } else {
        entry.set_password(value)?;
        Ok(())
    }
}

pub fn apply_keychain_to_settings(settings: &mut crate::settings::SettingsFile) {
    if settings.ai.xai_api_key.is_empty() {
        if let Some(v) = get_secret("xai_api_key") {
            settings.ai.xai_api_key = v;
        }
    }
    if settings.ai.firecrawl_api_key.is_empty() {
        if let Some(v) = get_secret("firecrawl_api_key") {
            settings.ai.firecrawl_api_key = v;
        }
    }
    if settings.catalog.nexar_client_id.is_empty() {
        if let Some(v) = get_secret("nexar_client_id") {
            settings.catalog.nexar_client_id = v;
        }
    }
    if settings.catalog.nexar_client_secret.is_empty() {
        if let Some(v) = get_secret("nexar_client_secret") {
            settings.catalog.nexar_client_secret = v;
        }
    }
}

pub fn persist_keychain_from_settings(settings: &crate::settings::SettingsFile) {
    let _ = set_secret("xai_api_key", &settings.ai.xai_api_key);
    let _ = set_secret("firecrawl_api_key", &settings.ai.firecrawl_api_key);
    let _ = set_secret("nexar_client_id", &settings.catalog.nexar_client_id);
    let _ = set_secret("nexar_client_secret", &settings.catalog.nexar_client_secret);
}

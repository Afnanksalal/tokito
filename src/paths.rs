//! Application data paths (local-first layout).

use std::env;
use std::path::{Path, PathBuf};

/// Directory containing the running executable (or `.` if unknown).
pub fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `%LOCALAPPDATA%/tokito` (Windows) or `~/.local/share/tokito` (Linux/macOS).
pub fn app_data_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tokito")
}

pub fn settings_path() -> PathBuf {
    app_data_root().join("settings.toml")
}

pub fn default_postgres_data_dir() -> PathBuf {
    app_data_root().join("postgres")
}

pub fn projects_root() -> PathBuf {
    app_data_root().join("projects")
}

pub fn project_dir(slug: &str) -> PathBuf {
    projects_root().join(slug)
}

pub fn project_exports_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join("exports")
}

pub fn project_toml_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join("project.toml")
}

pub fn slugify_name(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_underscore = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            slug.push('_');
            prev_underscore = true;
        }
    }
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

pub fn unique_project_dir(name: &str) -> PathBuf {
    let base = slugify_name(name);
    let root = projects_root();
    let candidate = root.join(&base);
    if !candidate.exists() {
        return candidate;
    }
    for i in 2..1000 {
        let alt = root.join(format!("{base}_{i}"));
        if !alt.exists() {
            return alt;
        }
    }
    root.join(format!("{base}_{}", uuid::Uuid::new_v4().simple()))
}

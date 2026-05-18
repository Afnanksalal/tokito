//! `project.toml` metadata for Tokito workspaces.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectToml {
    pub id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub database: ProjectDatabaseToml,
    #[serde(default)]
    pub exports: ProjectExportsToml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDatabaseToml {
    #[serde(default = "default_db_mode")]
    pub mode: String,
}

impl Default for ProjectDatabaseToml {
    fn default() -> Self {
        Self {
            mode: default_db_mode(),
        }
    }
}

fn default_db_mode() -> String {
    "global".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExportsToml {
    #[serde(default = "default_export_format")]
    pub default_format: String,
}

impl Default for ProjectExportsToml {
    fn default() -> Self {
        Self {
            default_format: default_export_format(),
        }
    }
}

fn default_export_format() -> String {
    "pdf".into()
}

impl Default for ProjectToml {
    fn default() -> Self {
        Self {
            id: None,
            name: "Project".into(),
            slug: "project".into(),
            database: ProjectDatabaseToml::default(),
            exports: ProjectExportsToml::default(),
        }
    }
}

impl ProjectToml {
    pub fn uses_embedded_db(&self) -> bool {
        self.database.mode.eq_ignore_ascii_case("embedded")
    }

    pub fn embedded_data_dir(workspace: &Path) -> PathBuf {
        workspace.join(".data").join("postgres")
    }
}

pub fn read(workspace: &Path) -> AppResult<ProjectToml> {
    let path = crate::paths::project_toml_path(workspace);
    if !path.is_file() {
        return Ok(ProjectToml {
            name: workspace
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Project")
                .to_string(),
            slug: workspace
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string(),
            ..Default::default()
        });
    }
    let text = fs::read_to_string(&path).map_err(|e| AppError::Any(e.into()))?;
    toml::from_str(&text).map_err(|e| AppError::BadRequest(format!("invalid project.toml: {e}")))
}

pub fn write(workspace: &Path, meta: &ProjectToml) -> AppResult<()> {
    let body = format!(
        r#"# Tokito project workspace
id = "{id}"
name = "{name}"
slug = "{slug}"

[database]
mode = "{mode}"

[exports]
default_format = "{fmt}"
"#,
        id = meta.id.map(|u| u.to_string()).unwrap_or_default(),
        name = meta.name.replace('"', "\\\""),
        slug = meta.slug,
        mode = meta.database.mode,
        fmt = meta.exports.default_format,
    );
    fs::write(crate::paths::project_toml_path(workspace), body)
        .map_err(|e| AppError::Any(e.into()))
}

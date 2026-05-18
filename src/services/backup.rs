//! Design backup archives on disk (export bundle and `pg_dump` when available).

use crate::error::{AppError, AppResult};
use crate::models::{SchematicDocument, SchematicView};
use chrono::Utc;
use std::path::{Path, PathBuf};

pub fn backup_dir_for_project(workspace: &Path) -> PathBuf {
    workspace.join("backups")
}

pub fn write_design_backup(
    workspace: &Path,
    design_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
) -> AppResult<PathBuf> {
    write_design_backup_inner(workspace, design_name, document, view, bom_csv, None)
}

/// Optional cluster dump when caller provides DB URL (global or project embedded).
pub fn write_design_backup_with_db(
    workspace: &Path,
    design_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
    database_url: Option<&str>,
    pg_embed_version: u16,
) -> AppResult<PathBuf> {
    let pg = database_url.map(|u| (u.to_string(), pg_embed_version));
    write_design_backup_inner(workspace, design_name, document, view, bom_csv, pg)
}

fn write_design_backup_inner(
    workspace: &Path,
    design_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
    pg_dump_context: Option<(String, u16)>,
) -> AppResult<PathBuf> {
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let safe: String = design_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let dir = backup_dir_for_project(workspace).join(format!("{safe}_{ts}"));
    let mcad = crate::services::mcad_export::document_handoff_json(document, design_name);
    crate::services::export_bundle::write_design_exports(
        &dir,
        &safe,
        document,
        view,
        bom_csv,
        Some(&mcad),
    )
    .map_err(|e| AppError::Any(e.into()))?;

    if let Some((url, pg_version)) = pg_dump_context {
        let sql_path = dir.join("database.sql");
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if let Err(e) = handle.block_on(crate::db::pg_backup::pg_dump_to_file(
                &url, &sql_path, pg_version,
            )) {
                tracing::warn!(%e, "pg_dump skipped for design backup");
            }
        }
    }

    Ok(dir)
}

pub fn list_backups(workspace: &Path) -> Vec<PathBuf> {
    let root = backup_dir_for_project(workspace);
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                out.push(e.path());
            }
        }
    }
    out.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    out
}

pub fn open_backups_folder(workspace: &Path) -> PathBuf {
    let dir = backup_dir_for_project(workspace);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

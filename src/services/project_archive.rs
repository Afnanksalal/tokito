//! Import/export whole project workspaces (P3).

use crate::error::{AppError, AppResult};
use crate::models::CreateDesign;
use crate::store::{designs, projects};
use serde_json::Value;
use sqlx::PgPool;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

const MANIFEST: &str = "project_manifest.json";
const MAX_ZIP_ENTRIES: usize = 2_000;
const MAX_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024;

struct ScopedTempDir {
    path: PathBuf,
}

impl ScopedTempDir {
    fn new(prefix: &str) -> AppResult<Self> {
        let path = std::env::temp_dir().join(format!("{prefix}_{}", Uuid::new_v4()));
        fs::create_dir_all(&path).map_err(|e| AppError::Any(e.into()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScopedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct ImportedDesign {
    name: String,
    description: Option<String>,
    document: Option<crate::models::SchematicDocument>,
}

pub async fn export_project_zip(
    pool: &PgPool,
    project_id: Uuid,
    user_id: Uuid,
    dest_zip: &Path,
    database_url: Option<&str>,
    pg_embed_version: u16,
) -> AppResult<()> {
    let project = projects::get(pool, project_id).await?;
    let workspace = PathBuf::from(&project.workspace_path);
    let design_rows = designs::list_for_project(pool, project_id, user_id, 500).await?;
    let mut manifest = serde_json::json!({
        "project": project,
        "designs": [],
    });
    let tmp = ScopedTempDir::new("tokito_proj_export")?;

    let mut design_entries = Vec::new();
    for d in &design_rows {
        let design_dir = tmp.path().join(d.id.to_string());
        fs::create_dir_all(&design_dir).map_err(|e| AppError::Any(e.into()))?;
        let doc = crate::store::schematic_document::get(pool, d.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("document missing for {}", d.id)))?;
        let view = crate::store::schematic::get_view(pool, d.id).await?;
        let csv = crate::store::bom::csv_export(pool, d.id).await?;
        let _ = crate::services::export_bundle::write_design_exports(
            &design_dir,
            "design",
            &doc,
            &view,
            &csv,
            Some(&crate::services::mcad_export::document_handoff_json(
                &doc, &d.name,
            )),
        )?;
        fs::write(
            design_dir.join("design.json"),
            serde_json::to_string_pretty(d).map_err(|e| AppError::Any(e.into()))?,
        )
        .map_err(|e| AppError::Any(e.into()))?;
        fs::write(
            design_dir.join("schematic_document.json"),
            serde_json::to_string_pretty(&doc).map_err(|e| AppError::Any(e.into()))?,
        )
        .map_err(|e| AppError::Any(e.into()))?;
        design_entries.push(serde_json::json!({
            "design": d,
            "archive_dir": d.id.to_string(),
        }));
    }
    manifest["designs"] = Value::Array(design_entries);
    fs::write(
        tmp.path().join(MANIFEST),
        serde_json::to_string_pretty(&manifest).map_err(|e| AppError::Any(e.into()))?,
    )
    .map_err(|e| AppError::Any(e.into()))?;
    if workspace.join("project.toml").is_file() {
        fs::copy(
            workspace.join("project.toml"),
            tmp.path().join("project.toml"),
        )
        .map_err(|e| AppError::Any(e.into()))?;
    }

    if let Some(url) = database_url {
        let sql_path = tmp.path().join("database.sql");
        if let Err(e) =
            crate::db::pg_backup::pg_dump_to_file(url, &sql_path, pg_embed_version).await
        {
            tracing::warn!(%e, "project export: pg_dump skipped");
        }
    }

    let file = File::create(dest_zip).map_err(|e| AppError::Any(e.into()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, tmp.path(), "", options)?;
    zip.finish().map_err(|e| AppError::Any(e.into()))?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> AppResult<()> {
    for entry in fs::read_dir(dir).map_err(|e| AppError::Any(e.into()))? {
        let entry = entry.map_err(|e| AppError::Any(e.into()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let zip_path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        if path.is_dir() {
            add_dir_to_zip(zip, &path, &zip_path, options)?;
        } else {
            let mut f = File::open(&path).map_err(|e| AppError::Any(e.into()))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)
                .map_err(|e| AppError::Any(e.into()))?;
            zip.start_file(zip_path, options)
                .map_err(|e| AppError::Any(e.into()))?;
            zip.write_all(&buf).map_err(|e| AppError::Any(e.into()))?;
        }
    }
    Ok(())
}

pub async fn import_project_zip(
    pool: &PgPool,
    zip_path: &Path,
    owner_user_id: Uuid,
) -> AppResult<Uuid> {
    let file = File::open(zip_path).map_err(|e| AppError::Any(e.into()))?;
    let archive = ZipArchive::new(file).map_err(|e| AppError::Any(e.into()))?;
    let extract = extract_archive(archive, "tokito_import")?;
    let manifest_path = extract.path().join(MANIFEST);
    let manifest: Value = if manifest_path.is_file() {
        let text = fs::read_to_string(&manifest_path).map_err(|e| AppError::Any(e.into()))?;
        serde_json::from_str(&text).map_err(|e| AppError::BadRequest(e.to_string()))?
    } else {
        return Err(AppError::BadRequest("project zip missing manifest".into()));
    };
    let pname = manifest
        .pointer("/project/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported project");
    let mut imported_designs = Vec::new();
    if let Some(arr) = manifest.get("designs").and_then(|v| v.as_array()) {
        for entry in arr {
            let old = entry.get("design").cloned().unwrap_or(Value::Null);
            let name = old
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Imported design");
            let desc = old.get("description").and_then(|v| v.as_str());
            let dir_name = entry
                .get("archive_dir")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let src = extract.path().join(dir_name);
            let document = if src.join("schematic_document.json").is_file() {
                let text = fs::read_to_string(src.join("schematic_document.json"))
                    .map_err(|e| AppError::Any(e.into()))?;
                Some(serde_json::from_str(&text).map_err(|e| AppError::BadRequest(e.to_string()))?)
            } else {
                None
            };
            imported_designs.push(ImportedDesign {
                name: name.to_string(),
                description: desc.map(String::from),
                document,
            });
        }
    }
    let project =
        projects::create(pool, crate::models::CreateProject { name: pname.into() }).await?;
    let workspace = PathBuf::from(&project.workspace_path);
    if extract.path().join("project.toml").is_file() {
        let _ = fs::copy(
            extract.path().join("project.toml"),
            workspace.join("project.toml"),
        );
    }
    for imported in imported_designs {
        let new_design = designs::create(
            pool,
            CreateDesign {
                name: imported.name,
                description: imported.description,
                project_id: Some(project.id),
            },
            owner_user_id,
        )
        .await?;
        if let Some(doc) = imported.document {
            let (replace, _) = doc.to_replace_schematic();
            crate::store::schematic::replace(pool, new_design.id, replace).await?;
            crate::store::schematic_document::upsert(pool, new_design.id, &doc).await?;
        }
    }
    Ok(project.id)
}

pub async fn restore_design_archive(
    pool: &PgPool,
    zip_path: &Path,
    project_id: Uuid,
    owner_user_id: Uuid,
) -> AppResult<Uuid> {
    let file = File::open(zip_path).map_err(|e| AppError::Any(e.into()))?;
    let archive = ZipArchive::new(file).map_err(|e| AppError::Any(e.into()))?;
    let extract = extract_archive(archive, "tokito_restore")?;
    let design_json = extract.path().join("design.json");
    let doc_json = extract.path().join("schematic_document.json");
    let name = if design_json.is_file() {
        let v: Value = serde_json::from_str(
            &fs::read_to_string(&design_json).map_err(|e| AppError::Any(e.into()))?,
        )
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
        v.get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("Restored design")
            .to_string()
    } else {
        "Restored design".into()
    };
    let document = if doc_json.is_file() {
        let text = fs::read_to_string(&doc_json).map_err(|e| AppError::Any(e.into()))?;
        Some(
            serde_json::from_str::<crate::models::SchematicDocument>(&text)
                .map_err(|e| AppError::BadRequest(e.to_string()))?,
        )
    } else {
        None
    };
    let design = designs::create(
        pool,
        CreateDesign {
            name,
            description: None,
            project_id: Some(project_id),
        },
        owner_user_id,
    )
    .await?;
    if let Some(doc) = document {
        let (replace, _) = doc.to_replace_schematic();
        crate::store::schematic::replace(pool, design.id, replace).await?;
        crate::store::schematic_document::upsert(pool, design.id, &doc).await?;
        if extract.path().join("design_bom.csv").is_file()
            || extract.path().join("test_bom.csv").is_file()
        {
            let bom_path = if extract.path().join("design_bom.csv").is_file() {
                extract.path().join("design_bom.csv")
            } else {
                extract.path().join("test_bom.csv")
            };
            let _ = bom_path;
        }
    }
    Ok(design.id)
}

fn extract_archive(mut archive: ZipArchive<File>, prefix: &str) -> AppResult<ScopedTempDir> {
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(AppError::BadRequest(format!(
            "zip has too many entries (max {MAX_ZIP_ENTRIES})"
        )));
    }
    let extract = ScopedTempDir::new(prefix)?;
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| AppError::Any(e.into()))?;
        total = total.saturating_add(file.size());
        if total > MAX_EXTRACTED_BYTES {
            return Err(AppError::BadRequest(format!(
                "zip expands beyond {MAX_EXTRACTED_BYTES} bytes"
            )));
        }
        let name = file
            .enclosed_name()
            .ok_or_else(|| AppError::BadRequest("invalid zip entry path".into()))?;
        let outpath = extract.path().join(name);
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| AppError::Any(e.into()))?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| AppError::Any(e.into()))?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| AppError::Any(e.into()))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| AppError::Any(e.into()))?;
        }
    }
    Ok(extract)
}

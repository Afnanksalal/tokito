use crate::error::{AppError, AppResult};
use crate::models::{CreateProject, PatchProject, Project};
use crate::paths;
use sqlx::PgPool;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_PROJECT_ID: Uuid = uuid::uuid!("00000000-0000-4000-8000-000000000001");

pub fn default_project_id() -> Uuid {
    DEFAULT_PROJECT_ID
}

pub async fn ensure_default_workspace(pool: &PgPool) -> AppResult<()> {
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM projects WHERE id = $1")
        .bind(DEFAULT_PROJECT_ID)
        .fetch_optional(pool)
        .await?;
    let workspace = paths::project_dir("default");
    fs::create_dir_all(&workspace).map_err(|e| AppError::Any(e.into()))?;
    fs::create_dir_all(paths::project_exports_dir(&workspace))
        .map_err(|e| AppError::Any(e.into()))?;
    if exists.is_some() {
        sqlx::query(
            r#"UPDATE projects SET workspace_path = $2, updated_at = now()
               WHERE id = $1 AND workspace_path = 'default'"#,
        )
        .bind(DEFAULT_PROJECT_ID)
        .bind(workspace.to_string_lossy().as_ref())
        .execute(pool)
        .await?;
        return Ok(());
    }
    sqlx::query(
        r#"
        INSERT INTO projects (id, name, slug, workspace_path)
        VALUES ($1, 'Default', 'default', $2)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(DEFAULT_PROJECT_ID)
    .bind(workspace.to_string_lossy().as_ref())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list(pool: &PgPool, limit: i64) -> AppResult<Vec<Project>> {
    let lim = limit.clamp(1, 200);
    sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, slug, workspace_path, created_at, updated_at
        FROM projects
        ORDER BY updated_at DESC
        LIMIT $1
        "#,
    )
    .bind(lim)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Project> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, slug, workspace_path, created_at, updated_at
        FROM projects WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("project not found".into()))
}

pub async fn upsert_existing(pool: &PgPool, project: &Project) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO projects (id, name, slug, workspace_path, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE SET
          name = EXCLUDED.name,
          slug = EXCLUDED.slug,
          workspace_path = EXCLUDED.workspace_path,
          updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(project.id)
    .bind(&project.name)
    .bind(&project.slug)
    .bind(&project.workspace_path)
    .bind(project.created_at)
    .bind(project.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create(pool: &PgPool, body: CreateProject) -> AppResult<Project> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("project name required".into()));
    }
    let workspace = paths::unique_project_dir(name);
    fs::create_dir_all(&workspace).map_err(|e| AppError::Any(e.into()))?;
    fs::create_dir_all(paths::project_exports_dir(&workspace))
        .map_err(|e| AppError::Any(e.into()))?;
    let slug = workspace
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (id, name, slug, workspace_path)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, slug, workspace_path, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(&slug)
    .bind(workspace.to_string_lossy().as_ref())
    .fetch_one(pool)
    .await?;
    let meta = crate::project_toml::ProjectToml {
        id: Some(row.id),
        name: row.name.clone(),
        slug: row.slug.clone(),
        ..Default::default()
    };
    crate::project_toml::write(&workspace, &meta)?;
    Ok(row)
}

pub async fn patch(pool: &PgPool, id: Uuid, body: PatchProject) -> AppResult<Project> {
    let current = get(pool, id).await?;
    let name = body
        .name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(current.name);
    let row = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET name = $2, updated_at = now()
        WHERE id = $1
        RETURNING id, name, slug, workspace_path, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&name)
    .fetch_one(pool)
    .await?;

    let workspace = PathBuf::from(&row.workspace_path);
    let mut meta = crate::project_toml::read(&workspace).unwrap_or_default();
    meta.id = Some(row.id);
    meta.name = row.name.clone();
    meta.slug = row.slug.clone();
    crate::project_toml::write(&workspace, &meta)?;
    Ok(row)
}

pub fn read_toml_for_workspace(workspace: &Path) -> crate::project_toml::ProjectToml {
    crate::project_toml::read(workspace).unwrap_or_default()
}

pub async fn workspace_path_for_design(pool: &PgPool, design_id: Uuid) -> AppResult<PathBuf> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT p.workspace_path
        FROM designs d
        JOIN projects p ON p.id = d.project_id
        WHERE d.id = $1
        "#,
    )
    .bind(design_id)
    .fetch_optional(pool)
    .await?;
    if let Some((path,)) = row {
        return Ok(PathBuf::from(path));
    }
    ensure_default_workspace(pool).await?;
    Ok(paths::project_dir("default"))
}

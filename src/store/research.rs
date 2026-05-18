//! Research artifacts attached to a design (scrapes, notes).

use crate::error::{AppError, AppResult};
use crate::models::DesignResearchArtifact;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Values allowed by `design_research_artifacts_kind_check` — keep in sync with migrations.
pub const KIND_FIRECRAWL_SCRAPE: &str = "firecrawl_scrape";
pub const KIND_FIRECRAWL_SEARCH: &str = "firecrawl_search";
pub const KIND_MANUAL_NOTE: &str = "manual_note";
pub const KIND_ANNOTATION: &str = "annotation";

fn validate_kind(kind: &str) -> AppResult<()> {
    match kind {
        KIND_FIRECRAWL_SCRAPE | KIND_FIRECRAWL_SEARCH | KIND_MANUAL_NOTE | KIND_ANNOTATION => Ok(()),
        _ => Err(AppError::BadRequest(format!(
            "invalid research artifact kind {kind:?}"
        ))),
    }
}

pub async fn list_for_design(
    pool: &PgPool,
    design_id: Uuid,
    limit: i64,
) -> AppResult<Vec<DesignResearchArtifact>> {
    let lim = limit.clamp(1, 500);
    sqlx::query_as::<_, DesignResearchArtifact>(
        r#"
        SELECT id, design_id, kind, title, source_url, content_text, metadata_json, created_at
        FROM design_research_artifacts
        WHERE design_id = $1
        ORDER BY COALESCE((metadata_json->>'pinned')::boolean, false) DESC, created_at DESC
        LIMIT $2
        "#,
    )
    .bind(design_id)
    .bind(lim)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn insert(
    pool: &PgPool,
    design_id: Uuid,
    kind: &str,
    title: Option<&str>,
    source_url: Option<&str>,
    content_text: &str,
    metadata_json: Value,
) -> AppResult<DesignResearchArtifact> {
    validate_kind(kind)?;
    sqlx::query_as::<_, DesignResearchArtifact>(
        r#"
        INSERT INTO design_research_artifacts
          (id, design_id, kind, title, source_url, content_text, metadata_json)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, design_id, kind, title, source_url, content_text, metadata_json, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(design_id)
    .bind(kind)
    .bind(title)
    .bind(source_url)
    .bind(content_text)
    .bind(metadata_json)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn get(pool: &PgPool, artifact_id: Uuid) -> AppResult<DesignResearchArtifact> {
    sqlx::query_as::<_, DesignResearchArtifact>(
        r#"
        SELECT id, design_id, kind, title, source_url, content_text, metadata_json, created_at
        FROM design_research_artifacts WHERE id = $1
        "#,
    )
    .bind(artifact_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("research artifact not found".into()))
}

pub async fn update_manual(
    pool: &PgPool,
    artifact_id: Uuid,
    title: Option<&str>,
    content_text: &str,
) -> AppResult<DesignResearchArtifact> {
    let row = get(pool, artifact_id).await?;
    if row.kind != KIND_MANUAL_NOTE {
        return Err(AppError::BadRequest(
            "only manual_note artifacts can be edited".into(),
        ));
    }
    sqlx::query_as::<_, DesignResearchArtifact>(
        r#"
        UPDATE design_research_artifacts
        SET title = $2, content_text = $3
        WHERE id = $1
        RETURNING id, design_id, kind, title, source_url, content_text, metadata_json, created_at
        "#,
    )
    .bind(artifact_id)
    .bind(title)
    .bind(content_text)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub fn normalize_query(q: &str) -> String {
    q.trim().to_lowercase()
}

pub async fn has_search_for_query(pool: &PgPool, design_id: Uuid, query: &str) -> AppResult<bool> {
    let n: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint FROM design_research_artifacts
        WHERE design_id = $1
          AND kind = $2
          AND metadata_json->>'search_query_norm' = $3
        "#,
    )
    .bind(design_id)
    .bind(KIND_FIRECRAWL_SEARCH)
    .bind(normalize_query(query))
    .fetch_one(pool)
    .await?;
    Ok(n.0 > 0)
}

pub async fn set_pinned(pool: &PgPool, artifact_id: Uuid, pinned: bool) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE design_research_artifacts
        SET metadata_json = metadata_json || jsonb_build_object('pinned', $2::boolean)
        WHERE id = $1
        "#,
    )
    .bind(artifact_id)
    .bind(pinned)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_annotation(
    pool: &PgPool,
    design_id: Uuid,
    parent_artifact_id: Uuid,
    content_text: &str,
) -> AppResult<DesignResearchArtifact> {
    let meta = serde_json::json!({ "parent_artifact_id": parent_artifact_id.to_string() });
    insert(
        pool,
        design_id,
        KIND_ANNOTATION,
        Some("Annotation"),
        None,
        content_text,
        meta,
    )
    .await
}

pub async fn delete_artifact(pool: &PgPool, artifact_id: Uuid) -> AppResult<()> {
    let n = sqlx::query(r#"DELETE FROM design_research_artifacts WHERE id = $1"#)
        .bind(artifact_id)
        .execute(pool)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound("research artifact not found".into()));
    }
    Ok(())
}


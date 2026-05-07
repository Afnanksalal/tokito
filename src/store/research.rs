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

fn validate_kind(kind: &str) -> AppResult<()> {
    match kind {
        KIND_FIRECRAWL_SCRAPE | KIND_FIRECRAWL_SEARCH | KIND_MANUAL_NOTE => Ok(()),
        _ => Err(AppError::BadRequest(format!(
            "invalid research artifact kind {kind:?} (expected one of {KIND_FIRECRAWL_SCRAPE}, {KIND_FIRECRAWL_SEARCH}, {KIND_MANUAL_NOTE})"
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
        ORDER BY created_at DESC
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

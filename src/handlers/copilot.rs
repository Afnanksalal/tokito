//! Design intent and research artifact handlers.

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateResearchNote, PatchResearchNote, PutDesignIntent, ScrapeResearchUrls, SearchResearchWeb,
};
use crate::router::AppState;
use crate::services::research_pipeline;
use crate::store::{designs as design_store, intent, research};
use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use serde_json::json;
use uuid::Uuid;

pub async fn get_intent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<crate::models::DesignIntent>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let row = intent::get(&state.pool, id)
        .await?
        .unwrap_or_else(|| intent::empty_intent(id));
    Ok(Json(row))
}

pub async fn put_intent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<PutDesignIntent>,
) -> AppResult<Json<crate::models::DesignIntent>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let constraints = body.constraints.unwrap_or_else(|| json!({}));
    if !constraints.is_object() {
        return Err(AppError::BadRequest(
            "constraints must be a JSON object".into(),
        ));
    }
    let row = intent::upsert(&state.pool, id, body.goal_text.trim(), constraints).await?;
    Ok(Json(row))
}

pub async fn list_research(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<crate::models::DesignResearchArtifact>>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let rows = research::list_for_design(&state.pool, id, 200).await?;
    Ok(Json(rows))
}

pub async fn scrape_research(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ScrapeResearchUrls>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let ids = research_pipeline::scrape_urls_into_design(
        &state,
        &state.pool,
        auth.user_id,
        id,
        &body.urls,
    )
    .await?;
    Ok(Json(json!({ "artifact_ids": ids, "count": ids.len() })))
}

pub async fn search_research(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<SearchResearchWeb>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let ids = research_pipeline::search_web_into_design(
        &state,
        &state.pool,
        auth.user_id,
        id,
        &body.query,
        body.limit,
    )
    .await?;
    Ok(Json(json!({ "artifact_ids": ids, "count": ids.len() })))
}

#[derive(serde::Deserialize)]
pub struct CreateResearchAnnotation {
    pub parent_artifact_id: Uuid,
    pub content_text: String,
}

pub async fn create_research_annotation(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateResearchAnnotation>,
) -> AppResult<Json<crate::models::DesignResearchArtifact>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let text = body.content_text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest("content_text required".into()));
    }
    let parent = research::get(&state.pool, body.parent_artifact_id).await?;
    if parent.design_id != id {
        return Err(AppError::NotFound("parent artifact not found".into()));
    }
    let row = research::insert_annotation(&state.pool, id, body.parent_artifact_id, text).await?;
    Ok(Json(row))
}

pub async fn create_research_note(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateResearchNote>,
) -> AppResult<Json<crate::models::DesignResearchArtifact>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let text = body.content_text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest("content_text required".into()));
    }
    let row = research::insert(
        &state.pool,
        id,
        research::KIND_MANUAL_NOTE,
        body.title.as_deref(),
        None,
        text,
        json!({}),
    )
    .await?;
    Ok(Json(row))
}

pub async fn patch_research_note(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((design_id, artifact_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<PatchResearchNote>,
) -> AppResult<Json<crate::models::DesignResearchArtifact>> {
    let _ = design_store::assert_visible(&state.pool, design_id, auth.user_id).await?;
    let row = research::get(&state.pool, artifact_id).await?;
    if row.design_id != design_id {
        return Err(AppError::NotFound("artifact not found".into()));
    }
    let content = body
        .content_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(row.content_text.as_str());
    let updated = research::update_manual(
        &state.pool,
        artifact_id,
        body.title.as_deref().or(row.title.as_deref()),
        content,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn delete_research_note(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((design_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> AppResult<axum::http::StatusCode> {
    let _ = design_store::assert_visible(&state.pool, design_id, auth.user_id).await?;
    let row = research::get(&state.pool, artifact_id).await?;
    if row.design_id != design_id {
        return Err(AppError::NotFound("artifact not found".into()));
    }
    research::delete_artifact(&state.pool, artifact_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

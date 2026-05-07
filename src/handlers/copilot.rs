//! Design intent + research artifact HTTP API.

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{PutDesignIntent, ScrapeResearchUrls, SearchResearchWeb};
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

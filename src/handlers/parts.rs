use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{CreatePart, PartSearchParams};
use crate::router::AppState;
use crate::store::parts;
use axum::extract::{Path, Query, State};
use axum::Extension;
use axum::Json;
use uuid::Uuid;

pub async fn create_part(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<CreatePart>,
) -> AppResult<Json<crate::models::Part>> {
    if body.mpn.trim().is_empty() {
        return Err(AppError::BadRequest("mpn is required".into()));
    }
    let row = parts::create(&state.pool, body).await?;
    Ok(Json(row))
}

pub async fn get_part(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<crate::models::Part>> {
    let row = parts::get_by_id(&state.pool, id).await?;
    Ok(Json(row))
}

pub async fn search_parts(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(params): Query<PartSearchParams>,
) -> AppResult<Json<Vec<crate::models::Part>>> {
    let rows = parts::search(&state.pool, params).await?;
    Ok(Json(rows))
}

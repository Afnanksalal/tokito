use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::CreateManufacturer;
use crate::router::AppState;
use crate::store::manufacturers;
use axum::extract::{Query, State};
use axum::Extension;
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

pub async fn create_mfg(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<CreateManufacturer>,
) -> AppResult<Json<crate::models::Manufacturer>> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let row = manufacturers::create(&state.pool, body).await?;
    Ok(Json(row))
}

pub async fn list_mfg(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<crate::models::Manufacturer>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows = manufacturers::list(&state.pool, limit).await?;
    Ok(Json(rows))
}

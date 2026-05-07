use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::models::PartOffer;
use crate::router::AppState;
use crate::services::offers_sync;
use crate::store::{offers, parts};
use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SyncOffersBody {
    #[serde(default)]
    pub sources: Vec<String>,
}

pub async fn list_part_offers(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<PartOffer>>> {
    let _ = parts::get_by_id(&state.pool, id).await?;
    let rows = offers::list_for_part(&state.pool, id).await?;
    Ok(Json(rows))
}

pub async fn sync_part_offers(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<SyncOffersBody>,
) -> AppResult<Json<serde_json::Value>> {
    let out = offers_sync::sync_part_offers_for_part(&state, id, body.sources).await?;
    Ok(Json(out))
}

//! External parts catalog search (LCSC / Nexar).

use crate::error::AppResult;
use crate::models::{CatalogSearchResponse, PartSearchParams};
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::Json;

pub async fn search_catalog(
    State(state): State<AppState>,
    Query(params): Query<PartSearchParams>,
) -> AppResult<Json<CatalogSearchResponse>> {
    let q = params.q.as_deref().unwrap_or("").trim();
    let limit = params.limit.unwrap_or(20) as usize;
    let hits = crate::services::catalog_search::search(&state, q, limit).await?;
    Ok(Json(CatalogSearchResponse {
        query: q.to_string(),
        hits,
    }))
}

//! Server-side proxies for xAI and Firecrawl (API keys never sent to clients).

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::router::AppState;
use crate::services::{firecrawl, xai};
use crate::store::account;
use axum::extract::State;
use axum::Extension;
use axum::Json;

pub async fn xai_chat_completions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    account::ensure_llm_quota(&state.pool, auth.user_id, 8192).await?;
    let v = xai::chat_completion(&state, body).await?;
    let (pt, ct) = xai::usage_tokens(&v);
    account::record_llm_usage(&state.pool, auth.user_id, pt, ct).await?;
    Ok(Json(v))
}

pub async fn firecrawl_scrape(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    account::ensure_scrape_quota(&state.pool, auth.user_id).await?;
    let v = firecrawl::scrape(&state, body).await?;
    account::record_scrape(&state.pool, auth.user_id).await?;
    Ok(Json(v))
}

/// Proxies [`firecrawl::search`] (body must include `"query"`). One scrape quota unit per call.
pub async fn firecrawl_search(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    account::ensure_scrape_quota(&state.pool, auth.user_id).await?;
    let v = firecrawl::search(&state, body).await?;
    account::record_scrape(&state.pool, auth.user_id).await?;
    Ok(Json(v))
}

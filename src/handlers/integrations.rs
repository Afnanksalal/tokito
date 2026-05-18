//! Server-side proxies for AI providers and Firecrawl (API keys never sent to clients).

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::router::AppState;
use crate::services::{firecrawl, llm};
use crate::store::account;
use axum::extract::State;
use axum::Extension;
use axum::Json;

pub async fn ai_chat_completions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let v = llm::metered_chat_completion(&state, &state.pool, auth.user_id, body, 8192).await?;
    Ok(Json(v))
}

pub async fn firecrawl_scrape(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    account::reserve_scrapes(&state.pool, auth.user_id, 1).await?;
    let v = firecrawl::scrape(&state, body).await?;
    Ok(Json(v))
}

/// Proxies [`firecrawl::search`] (body must include `"query"`). One scrape quota unit per call.
pub async fn firecrawl_search(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    account::reserve_scrapes(&state.pool, auth.user_id, 1).await?;
    let v = firecrawl::search(&state, body).await?;
    Ok(Json(v))
}

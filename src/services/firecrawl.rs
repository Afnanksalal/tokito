//! Firecrawl scrape + search API helpers.
//!
//! Scrape: `POST {base}/scrape` (often `/v1/scrape`).  
//! Search: `POST {root}/v2/search` with a `query` string — see
//! <https://docs.firecrawl.dev/features/search>.

use crate::error::{AppError, AppResult};
use crate::router::AppState;
use serde_json::Value;

fn join_base_path(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Host root without a trailing `/v1` or `/v2` segment so we can call `/v2/search`.
fn api_root(fc_base_url: &str) -> String {
    let b = fc_base_url.trim_end_matches('/');
    b.strip_suffix("/v1")
        .or_else(|| b.strip_suffix("/v2"))
        .unwrap_or(b)
        .to_string()
}

pub async fn scrape(state: &AppState, body: Value) -> AppResult<Value> {
    let Some(fc) = state.firecrawl.as_ref() else {
        return Err(AppError::Unavailable(
            crate::user_messages::FIRECRAWL_NOT_CONFIGURED.into(),
        ));
    };
    let has_url = body
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_url {
        return Err(AppError::BadRequest(
            "body must include a non-empty \"url\" string".into(),
        ));
    }
    let url = join_base_path(&fc.base_url, "scrape");
    let res = state
        .http
        .post(&url)
        .bearer_auth(&fc.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "Firecrawl {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))
}

/// Web search (`query` + optional `limit`, `scrapeOptions`, etc.).
/// Request shape matches [Firecrawl Search API](https://docs.firecrawl.dev/features/search).
pub async fn search(state: &AppState, body: Value) -> AppResult<Value> {
    let Some(fc) = state.firecrawl.as_ref() else {
        return Err(AppError::Unavailable(
            crate::user_messages::FIRECRAWL_NOT_CONFIGURED.into(),
        ));
    };
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if query.is_empty() {
        return Err(AppError::BadRequest(
            r#"body must include a non-empty "query" string"#.into(),
        ));
    }

    let url = format!("{}/v2/search", api_root(&fc.base_url));
    let res = state
        .http
        .post(&url)
        .bearer_auth(&fc.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "Firecrawl search {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))
}

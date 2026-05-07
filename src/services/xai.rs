//! xAI OpenAI-compatible chat completions (shared by HTTP handlers + agent).

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

pub fn usage_tokens(resp: &Value) -> (i64, i64) {
    let usage = resp.get("usage");
    let prompt = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let completion = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    (prompt, completion)
}

pub async fn chat_completion(state: &AppState, body: Value) -> AppResult<Value> {
    let Some(xai) = state.xai.as_ref() else {
        return Err(AppError::Unavailable(
            "xAI is not configured; set TOKITO_XAI_API_KEY".into(),
        ));
    };
    let url = join_base_path(&xai.base_url, "chat/completions");
    let res = state
        .http
        .post(&url)
        .bearer_auth(&xai.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "xAI {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))
}

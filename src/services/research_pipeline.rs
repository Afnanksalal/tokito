//! Scrape URLs or Firecrawl **web search** into `design_research_artifacts` (quotas + provenance).

use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::services::firecrawl;
use crate::store::account;
use crate::store::research::{self, KIND_FIRECRAWL_SCRAPE, KIND_FIRECRAWL_SEARCH};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_URLS_PER_REQUEST: usize = 12;
const MAX_CONTENT_CHARS: usize = 500_000;
const MAX_SEARCH_RESULTS: u32 = 10;

fn normalize_urls(raw: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for s in raw {
        let t = s.trim();
        if t.is_empty() {
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            continue;
        }
        if !out.iter().any(|x: &String| x == t) {
            out.push(t.to_string());
        }
        if out.len() >= MAX_URLS_PER_REQUEST {
            break;
        }
    }
    out
}

/// Extract human-readable text from a Firecrawl scrape JSON payload (shape varies by API version).
pub fn firecrawl_response_to_text(resp: &Value) -> String {
    if let Some(s) = resp.get("markdown").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(s) = resp.pointer("/data/markdown").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(s) = resp.get("content").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    serde_json::to_string_pretty(resp).unwrap_or_else(|_| resp.to_string())
}

fn clamp_content(s: &str) -> (String, bool) {
    let n = s.chars().count();
    let out: String = s.chars().take(MAX_CONTENT_CHARS).collect();
    let truncated = n > MAX_CONTENT_CHARS;
    (out, truncated)
}

fn append_search_item(item: &Value, out: &mut Vec<(Option<String>, Option<String>, String)>) {
    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
    let url = item.get("url").and_then(|v| v.as_str()).map(String::from);
    let content = item
        .get("markdown")
        .or_else(|| item.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if url.is_none() && content.is_empty() && title.is_none() {
        return;
    }
    let body = if !content.is_empty() {
        content
    } else if let Some(ref t) = title {
        t.clone()
    } else {
        String::new()
    };
    if body.is_empty() && url.is_none() {
        return;
    }
    out.push((title, url, body));
}

/// Normalize Firecrawl `/v2/search` JSON into `(title, url, text)` rows.
fn parse_search_results(resp: &Value) -> Vec<(Option<String>, Option<String>, String)> {
    let mut out = Vec::new();
    let Some(data) = resp.get("data") else {
        return out;
    };
    if let Some(arr) = data.as_array() {
        for item in arr {
            append_search_item(item, &mut out);
        }
        return out;
    }
    if let Some(web) = data.get("web").and_then(|v| v.as_array()) {
        for item in web {
            append_search_item(item, &mut out);
        }
    }
    out
}

/// Firecrawl web search + optional per-result scrape → research artifacts (each counts toward scrape quota).
pub async fn search_web_into_design(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    query: &str,
    limit: Option<u32>,
) -> AppResult<Vec<Uuid>> {
    let q = query.trim();
    if q.is_empty() {
        return Err(AppError::BadRequest(
            "search query must not be empty".into(),
        ));
    }
    let lim = limit.unwrap_or(5).clamp(1, MAX_SEARCH_RESULTS);
    account::reserve_scrapes(pool, user_id, lim as i32).await?;
    let body = json!({
        "query": q,
        "limit": lim,
        "scrapeOptions": { "formats": ["markdown"] },
    });
    let resp = firecrawl::search(state, body).await?;

    if resp.get("success").and_then(|v| v.as_bool()) == Some(false) {
        account::refund_scrapes(pool, user_id, lim as i32).await?;
        let msg = resp
            .pointer("/error")
            .or_else(|| resp.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("Firecrawl search failed");
        return Err(AppError::Upstream(msg.to_string()));
    }

    let rows = parse_search_results(&resp);
    if rows.is_empty() {
        account::refund_scrapes(pool, user_id, lim as i32).await?;
        return Err(AppError::BadRequest(
            "Firecrawl search returned no text results (try a different query)".into(),
        ));
    }
    account::refund_scrapes(pool, user_id, (lim as i32 - rows.len() as i32).max(0)).await?;

    let mut artifact_ids = Vec::new();
    for (title, url_opt, text_full) in rows {
        let (content_text, truncated) = clamp_content(&text_full);
        let meta = json!({
            "firecrawl_search": true,
            "search_query": query,
            "search_query_norm": research::normalize_query(query),
            "truncated_to_chars": MAX_CONTENT_CHARS,
            "truncated": truncated,
        });
        let row = research::insert(
            pool,
            design_id,
            KIND_FIRECRAWL_SEARCH,
            title.as_deref(),
            url_opt.as_deref(),
            &content_text,
            meta,
        )
        .await?;
        artifact_ids.push(row.id);
    }

    Ok(artifact_ids)
}

/// Scrape each URL via Firecrawl (counts toward user scrape quota) and append artifacts.
pub async fn scrape_urls_into_design(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    urls: &[String],
) -> AppResult<Vec<Uuid>> {
    let urls = normalize_urls(urls);
    if urls.is_empty() {
        return Err(AppError::BadRequest(
            "no valid http(s) URLs provided (max 12 per request)".into(),
        ));
    }

    let mut artifact_ids = Vec::new();
    account::reserve_scrapes(pool, user_id, urls.len() as i32).await?;

    for url in urls {
        let body = json!({
            "url": url,
            "formats": ["markdown"],
        });
        let resp = firecrawl::scrape(state, body).await?;

        let text_full = firecrawl_response_to_text(&resp);
        let (content_text, truncated) = clamp_content(&text_full);

        let title = resp
            .pointer("/metadata/title")
            .or_else(|| resp.pointer("/data/metadata/title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut meta = json!({
            "firecrawl_raw_keys": resp.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "truncated_to_chars": MAX_CONTENT_CHARS,
        });
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("truncated".into(), json!(truncated));
        }

        let row = research::insert(
            pool,
            design_id,
            KIND_FIRECRAWL_SCRAPE,
            title.as_deref(),
            Some(url.as_str()),
            &content_text,
            meta,
        )
        .await?;
        artifact_ids.push(row.id);
    }

    Ok(artifact_ids)
}

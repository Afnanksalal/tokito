//! Nexar Supply GraphQL (successor to Octopart API). Requires OAuth client credentials.

use crate::error::{AppError, AppResult};
use crate::models::UpsertOffer;
use crate::router::AppState;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

const SEARCH_MPN_QUERY: &str = r#"
query SupSearchMpn($q: String!, $limit: Int!) {
  supSearchMpn(q: $q, limit: $limit) {
    hits {
      part {
        mpn
        sellers {
          company { name }
          offers {
            sku
            inventoryLevel
            clickUrl
            prices { quantity price currency }
          }
        }
      }
    }
  }
}
"#;

fn nexar_offers_from_graphql(data: &Value) -> Vec<UpsertOffer> {
    let mut out = Vec::new();
    let hits = data
        .pointer("/data/supSearchMpn/hits")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    for hit in hits {
        let sellers = hit
            .pointer("/part/sellers")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        for seller in sellers {
            let company = seller
                .pointer("/company/name")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let offers = seller
                .pointer("/offers")
                .and_then(|o| o.as_array())
                .cloned()
                .unwrap_or_default();
            for off in offers {
                let sku = off
                    .pointer("/sku")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if sku.is_empty() {
                    continue;
                }
                let click = off
                    .pointer("/clickUrl")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                let stock = off
                    .pointer("/inventoryLevel")
                    .and_then(|x| x.as_i64())
                    .or_else(|| {
                        off.pointer("/inventoryLevel")
                            .and_then(|x| x.as_f64())
                            .map(|f| f as i64)
                    });
                let mut currency = "USD".to_string();
                let mut cents: Option<i64> = None;
                if let Some(prices) = off.pointer("/prices").and_then(|p| p.as_array()) {
                    let mut best_qty = i64::MAX;
                    for p in prices {
                        let qty = p.pointer("/quantity").and_then(|x| x.as_i64()).unwrap_or(1);
                        if qty <= best_qty && qty >= 1 {
                            best_qty = qty;
                            if let Some(price) = p.pointer("/price").and_then(|x| x.as_f64()) {
                                cents = Some((price * 100.0).round() as i64);
                            }
                            if let Some(cur) = p.pointer("/currency").and_then(|x| x.as_str()) {
                                currency = cur.to_string();
                            }
                        }
                    }
                }
                let distributor = format!("Nexar:{company}");
                out.push(UpsertOffer {
                    distributor,
                    sku,
                    product_url: click,
                    currency,
                    unit_price_cents: cents,
                    stock_qty: stock,
                });
            }
        }
    }
    out
}

pub async fn access_token(state: &AppState) -> AppResult<String> {
    let cfg = state
        .nexar
        .as_ref()
        .ok_or_else(|| AppError::Unavailable(crate::user_messages::NEXAR_NOT_CONFIGURED.into()))?;
    let cache = state
        .nexar_token_cache
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("Nexar token cache not initialized".into()))?;
    {
        let guard = cache
            .lock()
            .map_err(|_| AppError::Any(anyhow::anyhow!("nexar token mutex poisoned")))?;
        if let Some((tok, until)) = guard.as_ref() {
            if Instant::now() < *until {
                return Ok(tok.clone());
            }
        }
    }
    let body = [
        ("grant_type", "client_credentials"),
        ("client_id", cfg.client_id.as_str()),
        ("client_secret", cfg.client_secret.as_str()),
        ("scope", cfg.scope.as_str()),
    ];
    let res = state
        .http
        .post("https://identity.nexar.com/connect/token")
        .form(&body)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "Nexar OAuth {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    let tr: TokenResponse = serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))?;
    let ttl = Duration::from_secs(tr.expires_in.saturating_sub(120).max(60));
    let until = Instant::now() + ttl;
    {
        let mut guard = cache
            .lock()
            .map_err(|_| AppError::Any(anyhow::anyhow!("nexar token mutex poisoned")))?;
        *guard = Some((tr.access_token.clone(), until));
    }
    Ok(tr.access_token)
}

pub async fn search_mpn_offers(state: &AppState, mpn: &str) -> AppResult<Vec<UpsertOffer>> {
    let token = access_token(state).await?;
    let payload = json!({
        "query": SEARCH_MPN_QUERY,
        "variables": { "q": mpn, "limit": 15 }
    });
    let res = state
        .http
        .post("https://api.nexar.com/graphql")
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let v: Value = res.json().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "Nexar GraphQL HTTP {status}: {v}"
        )));
    }
    if let Some(errs) = v.get("errors").and_then(|e| e.as_array()) {
        if !errs.is_empty() {
            tracing::warn!(?errs, "Nexar GraphQL errors");
            return Err(AppError::Upstream(format!(
                "Nexar GraphQL errors: {}",
                serde_json::to_string(errs).unwrap_or_default()
            )));
        }
    }
    Ok(nexar_offers_from_graphql(&v))
}

pub async fn sync_offers_for_part(
    state: &AppState,
    part_id: uuid::Uuid,
    mpn: &str,
) -> AppResult<usize> {
    let offers = search_mpn_offers(state, mpn).await?;
    let n = offers.len();
    for o in offers {
        crate::store::offers::upsert(&state.pool, part_id, o).await?;
    }
    Ok(n)
}

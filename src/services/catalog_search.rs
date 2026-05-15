//! LCSC + Nexar catalog search for in-app part placement.

use crate::error::AppResult;
use crate::models::CatalogPartHit;
use crate::router::AppState;
use serde_json::Value;
use std::collections::HashMap;

const NEXAR_CATALOG_QUERY: &str = r#"
query SupSearchMpnCatalog($q: String!, $limit: Int!) {
  supSearchMpn(q: $q, limit: $limit) {
    hits {
      part {
        mpn
        name
        manufacturer { name }
        bestDatasheet { url }
        category { name }
        specs {
          attribute { name shortname }
          displayValue
        }
      }
    }
  }
}
"#;

pub async fn search(state: &AppState, query: &str, limit: usize) -> AppResult<Vec<CatalogPartHit>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let limit = limit.clamp(1, 40);
    let mut by_mpn: HashMap<String, CatalogPartHit> = HashMap::new();

    for hit in crate::services::lcsc::search_catalog(state, q, limit).await {
        let key = hit.mpn.to_ascii_uppercase();
        by_mpn.entry(key).or_insert(hit);
    }

    if state.nexar.is_some() {
        if let Ok(nexar_hits) = search_nexar_catalog(state, q, limit).await {
            for hit in nexar_hits {
                let key = hit.mpn.to_ascii_uppercase();
                by_mpn
                    .entry(key)
                    .and_modify(|existing| merge_catalog_hit(existing, &hit))
                    .or_insert(hit);
            }
        }
    }

    let mut out: Vec<_> = by_mpn.into_values().collect();
    out.sort_by(|a, b| a.mpn.cmp(&b.mpn));
    out.truncate(limit);
    Ok(out)
}

fn merge_catalog_hit(dst: &mut CatalogPartHit, src: &CatalogPartHit) {
    if dst.package_name.is_none() {
        dst.package_name = src.package_name.clone();
    }
    if dst.footprint_hint.is_none() {
        dst.footprint_hint = src.footprint_hint.clone();
    }
    if dst.datasheet_url.is_none() {
        dst.datasheet_url = src.datasheet_url.clone();
    }
    if dst.description.is_none() {
        dst.description = src.description.clone();
    }
    if dst.manufacturer.is_none() {
        dst.manufacturer = src.manufacturer.clone();
    }
}

async fn search_nexar_catalog(
    state: &AppState,
    q: &str,
    limit: usize,
) -> AppResult<Vec<CatalogPartHit>> {
    let token = crate::services::nexar::access_token(state).await?;
    let body = serde_json::json!({
        "query": NEXAR_CATALOG_QUERY,
        "variables": { "q": q, "limit": limit }
    });
    let res = state
        .http
        .post("https://api.nexar.com/graphql")
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| crate::error::AppError::Any(anyhow::anyhow!("Nexar HTTP: {e}")))?;
    let v: Value = res
        .json()
        .await
        .map_err(|e| crate::error::AppError::Any(anyhow::anyhow!("Nexar JSON: {e}")))?;
    Ok(parse_nexar_catalog(&v))
}

fn parse_nexar_catalog(data: &Value) -> Vec<CatalogPartHit> {
    let mut out = Vec::new();
    let Some(hits) = data
        .pointer("/data/supSearchMpn/hits")
        .and_then(|x| x.as_array())
    else {
        return out;
    };
    for hit in hits {
        let part = hit.get("part").unwrap_or(hit);
        let mpn = part
            .pointer("/mpn")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if mpn.is_empty() {
            continue;
        }
        let manufacturer = part
            .pointer("/manufacturer/name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let description = part
            .pointer("/name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let datasheet_url = part
            .pointer("/bestDatasheet/url")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let mut package_name = None;
        if let Some(specs) = part.get("specs").and_then(|s| s.as_array()) {
            for spec in specs {
                let attr = spec
                    .pointer("/attribute/name")
                    .or_else(|| spec.pointer("/attribute/shortname"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if attr.eq_ignore_ascii_case("case/package")
                    || attr.eq_ignore_ascii_case("package")
                    || attr.contains("Package")
                {
                    package_name = spec
                        .get("displayValue")
                        .and_then(|x| x.as_str())
                        .map(|s| s.trim().to_string());
                    break;
                }
            }
        }
        let footprint_hint = package_name
            .as_ref()
            .map(|p| crate::services::footprint_map::hint_from_package(p));
        out.push(CatalogPartHit {
            mpn: mpn.clone(),
            manufacturer,
            description,
            package_name,
            footprint_hint,
            datasheet_url,
            distributor: "Nexar".into(),
            sku: mpn,
            product_url: None,
            stock_qty: None,
            unit_price_cents: None,
            currency: None,
        });
    }
    out
}

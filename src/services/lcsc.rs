//! Best-effort LCSC discovery via public JSON endpoints (no signed partner API).

use crate::models::UpsertOffer;
use crate::router::AppState;
use serde_json::Value;

pub async fn search_offers(state: &AppState, mpn: &str) -> Vec<UpsertOffer> {
    if !state.lcsc_anonymous_search {
        return Vec::new();
    }
    let url = format!(
        "https://lcsc.com/api/global/additional/search?q={}",
        urlencoding::encode(mpn)
    );
    let Ok(res) = state.http.get(&url).send().await else {
        return Vec::new();
    };
    let Ok(v) = res.json::<Value>().await else {
        return Vec::new();
    };
    extract_offers_from_lcsc_json(&v, mpn)
}

fn extract_offers_from_lcsc_json(v: &Value, q: &str) -> Vec<UpsertOffer> {
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for item in arr {
            push_if_product(item, &mut out);
        }
    } else if let Some(obj) = v.as_object() {
        for (_, val) in obj {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    push_if_product(item, &mut out);
                }
            } else {
                push_if_product(val, &mut out);
            }
        }
    }
    if out.is_empty() && !q.is_empty() {
        out.push(UpsertOffer {
            distributor: "LCSC".to_string(),
            sku: q.to_string(),
            product_url: Some(format!(
                "https://www.lcsc.com/search?q={}",
                urlencoding::encode(q)
            )),
            currency: "USD".to_string(),
            unit_price_cents: None,
            stock_qty: None,
        });
    }
    out
}

fn push_if_product(item: &Value, out: &mut Vec<UpsertOffer>) {
    let code = item
        .get("productCode")
        .or_else(|| item.get("componentId"))
        .or_else(|| item.get("product_id"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let Some(sku) = code else { return };
    let url = item
        .pointer("/productUrl")
        .or_else(|| item.pointer("/url"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some(format!("https://www.lcsc.com/product/{sku}.html")));
    out.push(UpsertOffer {
        distributor: "LCSC".to_string(),
        sku,
        product_url: url,
        currency: "USD".to_string(),
        unit_price_cents: None,
        stock_qty: item.get("stock").and_then(|x| x.as_i64()),
    });
}

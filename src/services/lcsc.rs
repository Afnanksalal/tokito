//! Best-effort LCSC discovery via public JSON endpoints (no signed partner API).
//!
//! Official partner API: https://lcsc.com/docs/openapi/ (requires LCSC account + key).
//! This module uses the same undocumented search surface as [jlcparts](https://github.com/yaqwsx/jlcparts).

use crate::models::{CatalogPartHit, UpsertOffer};
use crate::router::AppState;
use crate::services::footprint_map;
use serde_json::Value;

/// Catalog rows for search and footprint hints.
pub async fn search_catalog(state: &AppState, mpn: &str, limit: usize) -> Vec<CatalogPartHit> {
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
    extract_catalog_from_lcsc_json(&v, limit)
}

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

fn extract_catalog_from_lcsc_json(v: &Value, limit: usize) -> Vec<CatalogPartHit> {
    let mut out = Vec::new();
    let mut visit = |item: &Value| {
        if out.len() >= limit {
            return;
        }
        push_catalog_item(item, &mut out);
    };
    if let Some(arr) = v.as_array() {
        for item in arr {
            visit(item);
        }
    } else if let Some(obj) = v.as_object() {
        for (_, val) in obj {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    visit(item);
                }
            } else {
                visit(val);
            }
        }
    }
    out
}

fn push_catalog_item(item: &Value, out: &mut Vec<CatalogPartHit>) {
    let mpn = item
        .get("productModel")
        .or_else(|| item.get("title"))
        .or_else(|| item.get("productName"))
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let Some(mpn) = mpn else {
        return;
    };
    let sku = item
        .get("productCode")
        .or_else(|| item.get("componentId"))
        .and_then(|x| x.as_str())
        .unwrap_or(&mpn)
        .to_string();
    let package_name = item
        .get("package")
        .or_else(|| item.get("encapStandard"))
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let footprint_hint = package_name
        .as_ref()
        .map(|p| footprint_map::hint_from_package(p));
    let manufacturer = item
        .get("brandNameEn")
        .or_else(|| item.get("brandName"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let description = item
        .get("productIntroEn")
        .or_else(|| item.get("productIntro"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let datasheet_url = item
        .get("pdfUrl")
        .or_else(|| item.get("datasheet"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let stock_qty = item.get("stock").and_then(|x| x.as_i64());
    let unit_price_cents = item
        .pointer("/productPriceList/0/productPrice")
        .or_else(|| item.get("productPrice"))
        .and_then(|x| x.as_f64())
        .map(|p| (p * 100.0).round() as i64);
    let product_url = Some(format!("https://www.lcsc.com/product/{sku}.html"));
    out.push(CatalogPartHit {
        mpn,
        manufacturer,
        description,
        package_name,
        footprint_hint,
        datasheet_url,
        distributor: "LCSC".into(),
        sku,
        product_url,
        stock_qty,
        unit_price_cents,
        currency: Some("USD".into()),
    });
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

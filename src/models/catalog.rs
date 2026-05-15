//! Distributor catalog search results.

use serde::Serialize;

/// One purchasable part from an external catalog (not yet in local `parts` table).
#[derive(Debug, Clone, Serialize)]
pub struct CatalogPartHit {
    pub mpn: String,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    /// Package string from distributor (e.g. `SOT-23-5`, `0805`).
    pub package_name: Option<String>,
    /// Suggested footprint id (`Library:FootprintName`) when known.
    pub footprint_hint: Option<String>,
    pub datasheet_url: Option<String>,
    pub distributor: String,
    pub sku: String,
    pub product_url: Option<String>,
    pub stock_qty: Option<i64>,
    pub unit_price_cents: Option<i64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogSearchResponse {
    pub query: String,
    pub hits: Vec<CatalogPartHit>,
}

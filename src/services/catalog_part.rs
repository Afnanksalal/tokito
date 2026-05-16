//! Import distributor catalog hits into the local parts table.

use crate::error::AppResult;
use crate::models::{CatalogPartHit, CreatePart, Part};
use crate::store::{manufacturers, parts};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const CATALOG_MFR_SLUG: &str = "catalog";

/// Ensure a row exists for a catalog / LCSC hit and return it.
pub async fn ensure_part_from_catalog_hit(pool: &PgPool, hit: &CatalogPartHit) -> AppResult<Part> {
    let manufacturer_id = ensure_catalog_manufacturer(pool).await?;
    let mpn = hit.mpn.trim();
    if let Some(existing) = parts::find_by_manufacturer_and_mpn(pool, manufacturer_id, mpn).await? {
        return Ok(existing);
    }
    let mut attrs = json!({
        "distributor": hit.distributor,
        "sku": hit.sku,
    });
    if let Some(url) = &hit.product_url {
        attrs["product_url"] = json!(url);
    }
    if let Some(url) = &hit.datasheet_url {
        attrs["datasheet_url"] = json!(url);
    }
    parts::create(
        pool,
        CreatePart {
            manufacturer_id,
            mpn: mpn.to_string(),
            description: hit.description.clone().or_else(|| hit.manufacturer.clone()),
            package_name: hit.package_name.clone(),
            attributes: Some(attrs),
        },
    )
    .await
}

async fn ensure_catalog_manufacturer(pool: &PgPool) -> AppResult<Uuid> {
    if let Some(m) = manufacturers::get_by_slug(pool, CATALOG_MFR_SLUG).await? {
        return Ok(m.id);
    }
    let row = manufacturers::create(
        pool,
        crate::models::CreateManufacturer {
            name: "Catalog".into(),
            slug: Some(CATALOG_MFR_SLUG.into()),
        },
    )
    .await?;
    Ok(row.id)
}

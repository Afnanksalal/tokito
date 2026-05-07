use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PartOffer {
    pub id: Uuid,
    pub part_id: Uuid,
    pub distributor: String,
    pub sku: String,
    pub product_url: Option<String>,
    pub currency: String,
    pub unit_price_cents: Option<i64>,
    pub stock_qty: Option<i64>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertOffer {
    pub distributor: String,
    pub sku: String,
    pub product_url: Option<String>,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub unit_price_cents: Option<i64>,
    pub stock_qty: Option<i64>,
}

fn default_currency() -> String {
    "USD".to_string()
}

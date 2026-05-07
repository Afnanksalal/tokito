use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Part {
    pub id: Uuid,
    pub manufacturer_id: Uuid,
    pub mpn: String,
    pub description: Option<String>,
    pub package_name: Option<String>,
    #[sqlx(json)]
    pub attributes: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePart {
    pub manufacturer_id: Uuid,
    pub mpn: String,
    pub description: Option<String>,
    pub package_name: Option<String>,
    #[serde(default)]
    pub attributes: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct PartSearchParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

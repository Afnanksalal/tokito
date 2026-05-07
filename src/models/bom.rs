use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BomLine {
    pub id: Uuid,
    pub design_id: Uuid,
    pub part_id: Uuid,
    pub quantity: f64,
    pub sort_order: i32,
    pub notes: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BomLineInput {
    pub part_id: Uuid,
    pub quantity: f64,
    #[serde(default)]
    pub sort_order: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReplaceBom {
    pub lines: Vec<BomLineInput>,
}

#[derive(Debug, Deserialize)]
pub struct AppendBom {
    pub lines: Vec<BomLineInput>,
}

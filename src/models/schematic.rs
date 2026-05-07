use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SchematicInstance {
    pub id: Uuid,
    pub design_id: Uuid,
    pub part_id: Option<Uuid>,
    pub ref_des: String,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
    pub rotation: f64,
    #[sqlx(json)]
    pub meta: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SchematicNet {
    pub id: Uuid,
    pub design_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SchematicPin {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub pin_name: String,
    pub net_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SchematicView {
    pub instances: Vec<SchematicInstance>,
    pub nets: Vec<SchematicNet>,
    pub pins: Vec<SchematicPin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceSchematic {
    pub instances: Vec<SchematicInstanceInput>,
    pub nets: Vec<SchematicNetInput>,
    pub pins: Vec<SchematicPinInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicInstanceInput {
    pub id: Option<Uuid>,
    pub part_id: Option<Uuid>,
    pub ref_des: String,
    pub position: Option<Position>,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicNetInput {
    pub id: Option<Uuid>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicPinInput {
    pub instance_ref: String,
    pub pin_name: String,
    pub net_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

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

impl SchematicView {
    /// Build an in-memory view from a replace payload (e.g. unsaved editor export).
    pub fn from_replace(design_id: Uuid, body: &ReplaceSchematic) -> Self {
        use std::collections::HashMap;

        let now = Utc::now();
        let mut ref_to_id: HashMap<String, Uuid> = HashMap::new();
        let instances: Vec<SchematicInstance> = body
            .instances
            .iter()
            .map(|i| {
                let id = i.id.unwrap_or_else(Uuid::new_v4);
                ref_to_id.insert(i.ref_des.clone(), id);
                let (px, py) = match &i.position {
                    Some(p) => (Some(p.x), Some(p.y)),
                    None => (None, None),
                };
                SchematicInstance {
                    id,
                    design_id,
                    part_id: i.part_id,
                    ref_des: i.ref_des.clone(),
                    pos_x: px,
                    pos_y: py,
                    rotation: i.rotation,
                    meta: i.meta.clone().unwrap_or_else(|| serde_json::json!({})),
                    created_at: now,
                }
            })
            .collect();

        let mut net_name_to_id: HashMap<String, Uuid> = body
            .nets
            .iter()
            .map(|n| {
                let id = n.id.unwrap_or_else(Uuid::new_v4);
                (n.name.clone(), id)
            })
            .collect();
        for pin in &body.pins {
            net_name_to_id
                .entry(pin.net_name.clone())
                .or_insert_with(Uuid::new_v4);
        }
        let nets: Vec<SchematicNet> = net_name_to_id
            .iter()
            .map(|(name, id)| SchematicNet {
                id: *id,
                design_id,
                name: name.clone(),
                created_at: now,
            })
            .collect();

        let pins: Vec<SchematicPin> = body
            .pins
            .iter()
            .filter_map(|p| {
                let instance_id = *ref_to_id.get(&p.instance_ref)?;
                let net_id = *net_name_to_id.get(&p.net_name)?;
                Some(SchematicPin {
                    id: Uuid::new_v4(),
                    instance_id,
                    pin_name: p.pin_name.clone(),
                    net_id,
                    created_at: now,
                })
            })
            .collect();

        Self {
            instances,
            nets,
            pins,
        }
    }
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

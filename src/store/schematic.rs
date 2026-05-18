use crate::error::{AppError, AppResult};
use crate::models::{
    ReplaceSchematic, SchematicInstance, SchematicNet, SchematicPin, SchematicView,
};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn get_view(pool: &PgPool, design_id: Uuid) -> AppResult<SchematicView> {
    let instances = sqlx::query_as::<_, SchematicInstance>(
        r#"
        SELECT id, design_id, part_id, ref_des, pos_x, pos_y, rotation, meta, created_at
        FROM schematic_instances WHERE design_id = $1 ORDER BY ref_des ASC
        "#,
    )
    .bind(design_id)
    .fetch_all(pool)
    .await?;

    let nets = sqlx::query_as::<_, SchematicNet>(
        r#"SELECT id, design_id, name, created_at FROM schematic_nets WHERE design_id = $1 ORDER BY name ASC"#,
    )
    .bind(design_id)
    .fetch_all(pool)
    .await?;

    let pins = if instances.is_empty() {
        vec![]
    } else {
        let ids: Vec<Uuid> = instances.iter().map(|i| i.id).collect();
        sqlx::query_as::<_, SchematicPin>(
            r#"
            SELECT id, instance_id, pin_name, net_id, created_at
            FROM schematic_pins
            WHERE instance_id = ANY($1)
            ORDER BY pin_name ASC
            "#,
        )
        .bind(&ids)
        .fetch_all(pool)
        .await?
    };

    Ok(SchematicView {
        instances,
        nets,
        pins,
    })
}

/// Replaces only the normalized schematic graph for a design.
///
/// Rich schematic documents are persisted separately through
/// `store::schematic_document`. Keeping these writes separate prevents graph
/// compatibility saves from destroying document-only data such as symbol
/// fields, pin layout, sheets, text, and ERC markers.
pub async fn replace(pool: &PgPool, design_id: Uuid, body: ReplaceSchematic) -> AppResult<()> {
    crate::services::schematic_validate::validate_topology(&body)?;
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"DELETE FROM schematic_pins WHERE instance_id IN
           (SELECT id FROM schematic_instances WHERE design_id = $1)"#,
    )
    .bind(design_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(r#"DELETE FROM schematic_nets WHERE design_id = $1"#)
        .bind(design_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(r#"DELETE FROM schematic_instances WHERE design_id = $1"#)
        .bind(design_id)
        .execute(&mut *tx)
        .await?;

    let mut ref_to_id: HashMap<String, Uuid> = HashMap::new();

    for inst in &body.instances {
        let id = inst.id.unwrap_or_else(Uuid::new_v4);
        let meta = inst.meta.clone().unwrap_or_else(|| json!({}));
        let (px, py) = match &inst.position {
            Some(p) => (Some(p.x), Some(p.y)),
            None => (None, None),
        };
        sqlx::query(
            r#"
            INSERT INTO schematic_instances
                (id, design_id, part_id, ref_des, pos_x, pos_y, rotation, meta)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(design_id)
        .bind(inst.part_id)
        .bind(&inst.ref_des)
        .bind(px)
        .bind(py)
        .bind(inst.rotation)
        .bind(meta)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
                AppError::Conflict("duplicate ref_des in schematic".into())
            }
            e => e.into(),
        })?;
        if ref_to_id.insert(inst.ref_des.clone(), id).is_some() {
            return Err(AppError::BadRequest(
                "duplicate ref_des in request payload".into(),
            ));
        }
    }

    let mut net_name_to_id: HashMap<String, Uuid> = HashMap::new();
    for net in &body.nets {
        let id = net.id.unwrap_or_else(Uuid::new_v4);
        sqlx::query(r#"INSERT INTO schematic_nets (id, design_id, name) VALUES ($1, $2, $3)"#)
            .bind(id)
            .bind(design_id)
            .bind(&net.name)
            .execute(&mut *tx)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
                    AppError::Conflict("duplicate net name in schematic".into())
                }
                e => e.into(),
            })?;
        if net_name_to_id.insert(net.name.clone(), id).is_some() {
            return Err(AppError::BadRequest(
                "duplicate net name in request payload".into(),
            ));
        }
    }

    for pin in &body.pins {
        let iid = ref_to_id.get(&pin.instance_ref).ok_or_else(|| {
            AppError::BadRequest(format!("unknown instance_ref: {}", pin.instance_ref))
        })?;
        let nid = net_name_to_id
            .get(&pin.net_name)
            .ok_or_else(|| AppError::BadRequest(format!("unknown net name: {}", pin.net_name)))?;
        sqlx::query(
            r#"
            INSERT INTO schematic_pins (id, instance_id, pin_name, net_id)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(iid)
        .bind(&pin.pin_name)
        .bind(nid)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
                AppError::Conflict("duplicate pin on instance".into())
            }
            e => e.into(),
        })?;
    }

    tx.commit().await.map_err(Into::into)
}

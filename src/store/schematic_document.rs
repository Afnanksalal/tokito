use crate::error::{AppError, AppResult};
use crate::models::{SchematicDocument, SCHEMATIC_DOCUMENT_SCHEMA_VERSION};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get(pool: &PgPool, design_id: Uuid) -> AppResult<Option<SchematicDocument>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"
        SELECT document_json
        FROM design_schematic_documents
        WHERE design_id = $1
        "#,
    )
    .bind(design_id)
    .fetch_optional(pool)
    .await?;

    let doc: Option<SchematicDocument> = match row {
        None => None,
        Some(v) => Some(
            serde_json::from_value(v).map_err(|e| AppError::Any(anyhow::anyhow!(e)))?,
        ),
    };
    let Some(mut doc) = doc else {
        return Ok(None);
    };
    let needs_save = doc.schema_version < SCHEMATIC_DOCUMENT_SCHEMA_VERSION;
    doc = SchematicDocument::upgrade_to_current(doc);
    if needs_save {
        upsert(pool, design_id, &doc).await?;
    }
    Ok(Some(doc))
}

pub async fn upsert(pool: &PgPool, design_id: Uuid, document: &SchematicDocument) -> AppResult<()> {
    let value = serde_json::to_value(document).map_err(|e| anyhow::anyhow!(e))?;
    sqlx::query(
        r#"
        INSERT INTO design_schematic_documents
            (design_id, document_json, schema_version, updated_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (design_id) DO UPDATE SET
            document_json = EXCLUDED.document_json,
            schema_version = EXCLUDED.schema_version,
            updated_at = now()
        "#,
    )
    .bind(design_id)
    .bind(value)
    .bind(document.schema_version as i32)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, design_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        DELETE FROM design_schematic_documents
        WHERE design_id = $1
        "#,
    )
    .bind(design_id)
    .execute(pool)
    .await?;
    Ok(())
}

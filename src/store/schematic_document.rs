use crate::error::AppResult;
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

    row.map(serde_json::from_value)
        .transpose()
        .map_err(|e| anyhow::anyhow!(e).into())
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
    .bind(SCHEMATIC_DOCUMENT_SCHEMA_VERSION as i32)
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

use crate::error::{AppError, AppResult};
use crate::models::{CreatePart, Part, PartSearchParams};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn create(pool: &PgPool, body: CreatePart) -> AppResult<Part> {
    let attrs = body.attributes.unwrap_or_else(|| json!({}));
    let row = sqlx::query_as::<_, Part>(
        r#"
        INSERT INTO parts (id, manufacturer_id, mpn, description, package_name, attributes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, manufacturer_id, mpn, description, package_name, attributes,
                  created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(body.manufacturer_id)
    .bind(&body.mpn)
    .bind(&body.description)
    .bind(&body.package_name)
    .bind(attrs)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
            AppError::Conflict("part already exists for this manufacturer (mpn)".into())
        }
        sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23503") => {
            AppError::BadRequest("unknown manufacturer_id".into())
        }
        e => e.into(),
    })?;
    Ok(row)
}

pub async fn find_by_manufacturer_and_mpn(
    pool: &PgPool,
    manufacturer_id: Uuid,
    mpn: &str,
) -> AppResult<Option<Part>> {
    sqlx::query_as::<_, Part>(
        r#"
        SELECT id, manufacturer_id, mpn, description, package_name, attributes, created_at, updated_at
        FROM parts
        WHERE manufacturer_id = $1 AND lower(trim(mpn)) = lower(trim($2))
        LIMIT 1
        "#,
    )
    .bind(manufacturer_id)
    .bind(mpn)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<Part> {
    sqlx::query_as::<_, Part>(
        r#"SELECT id, manufacturer_id, mpn, description, package_name, attributes, created_at, updated_at
           FROM parts WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("part not found".into()))
}

pub async fn get_by_ids(pool: &PgPool, ids: &[Uuid]) -> AppResult<HashMap<Uuid, Part>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, Part>(
        r#"SELECT id, manufacturer_id, mpn, description, package_name, attributes, created_at, updated_at
           FROM parts WHERE id = ANY($1)"#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|p| (p.id, p)).collect())
}

pub async fn search(pool: &PgPool, params: PartSearchParams) -> AppResult<Vec<Part>> {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return sqlx::query_as::<_, Part>(
            r#"SELECT id, manufacturer_id, mpn, description, package_name, attributes, created_at, updated_at
               FROM parts ORDER BY mpn ASC LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(Into::into);
    }
    let like = format!("%{}%", q.trim());
    sqlx::query_as::<_, Part>(
        r#"
        SELECT id, manufacturer_id, mpn, description, package_name, attributes, created_at, updated_at
        FROM parts
        WHERE mpn ILIKE $1 OR description ILIKE $1
        ORDER BY mpn ASC
        LIMIT $2
        "#,
    )
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

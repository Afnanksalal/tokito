use crate::error::{AppError, AppResult};
use crate::models::{CreateDesign, Design, PatchDesign};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(pool: &PgPool, body: CreateDesign, owner_user_id: Uuid) -> AppResult<Design> {
    sqlx::query_as::<_, Design>(
        r#"
        INSERT INTO designs (id, name, description, owner_user_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, owner_user_id, created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&body.name)
    .bind(&body.description)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Design> {
    sqlx::query_as::<_, Design>(
        r#"SELECT id, name, description, owner_user_id, created_at, updated_at FROM designs WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("design not found".into()))
}

pub fn visible_to_user(row: &Design, user_id: Uuid) -> bool {
    row.owner_user_id.is_none() || row.owner_user_id == Some(user_id)
}

pub async fn assert_visible(pool: &PgPool, id: Uuid, user_id: Uuid) -> AppResult<Design> {
    let row = get(pool, id).await?;
    if !visible_to_user(&row, user_id) {
        return Err(AppError::Forbidden("design not accessible".into()));
    }
    Ok(row)
}

pub async fn list_for_user(pool: &PgPool, user_id: Uuid, limit: i64) -> AppResult<Vec<Design>> {
    let lim = limit.clamp(1, 200);
    sqlx::query_as::<_, Design>(
        r#"
        SELECT id, name, description, owner_user_id, created_at, updated_at
        FROM designs
        WHERE owner_user_id IS NULL OR owner_user_id = $1
        ORDER BY updated_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(lim)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn patch(pool: &PgPool, id: Uuid, patch: PatchDesign) -> AppResult<Design> {
    let current = get(pool, id).await?;
    let name = patch.name.unwrap_or(current.name);
    let description = patch.description.or(current.description);
    sqlx::query_as::<_, Design>(
        r#"
        UPDATE designs SET name = $2, description = $3, updated_at = now()
        WHERE id = $1
        RETURNING id, name, description, owner_user_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

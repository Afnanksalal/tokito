use crate::error::{AppError, AppResult};
use crate::models::{CreateManufacturer, Manufacturer};
use sqlx::PgPool;
use uuid::Uuid;

/// Creates a manufacturer; slug defaults to a slugified `name` when omitted.
pub async fn create(pool: &PgPool, body: CreateManufacturer) -> AppResult<Manufacturer> {
    let slug = match body.slug {
        Some(s) if !s.is_empty() => s,
        _ => slugify(&body.name),
    };
    let row = sqlx::query_as::<_, Manufacturer>(
        r#"
        INSERT INTO manufacturers (id, name, slug)
        VALUES ($1, $2, $3)
        RETURNING id, name, slug, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&body.name)
    .bind(&slug)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
            AppError::Conflict("manufacturer slug already exists".into())
        }
        e => e.into(),
    })?;
    Ok(row)
}

pub async fn get_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<Manufacturer>> {
    sqlx::query_as::<_, Manufacturer>(
        r#"SELECT id, name, slug, created_at FROM manufacturers WHERE slug = $1"#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn list(pool: &PgPool, limit: i64) -> AppResult<Vec<Manufacturer>> {
    let rows = sqlx::query_as::<_, Manufacturer>(
        r#"SELECT id, name, slug, created_at FROM manufacturers ORDER BY name ASC LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "part".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("STMicro"), "stmicro");
        assert_eq!(slugify("ACME  Parts Co."), "acme-parts-co");
    }
}

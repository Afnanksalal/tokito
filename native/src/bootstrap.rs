//! Native single-user bootstrap (no auth UI yet).

use anyhow::Context;
use uuid::Uuid;

pub async fn ensure_local_user(pool: &sqlx::PgPool) -> anyhow::Result<Uuid> {
    let email = "local@tokito";
    if let Some(u) = tokito::store::account::find_user_by_email(pool, email).await? {
        return Ok(u.id);
    }
    let hash = bcrypt::hash("disabled", 4).context("bcrypt")?;
    let u = tokito::store::account::create_user(pool, email, &hash, Some("Local")).await?;
    Ok(u.id)
}

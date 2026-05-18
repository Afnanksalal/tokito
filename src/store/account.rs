//! Users, API keys, usage quotas, agent run audit.

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub quota_llm_tokens_monthly: i64,
    pub quota_scrapes_daily: i32,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: Option<&str>,
) -> AppResult<UserRow> {
    sqlx::query_as::<_, UserRow>(
        r#"
        INSERT INTO users (email, password_hash, display_name)
        VALUES ($1, $2, $3)
        RETURNING id, email, password_hash, quota_llm_tokens_monthly, quota_scrapes_daily
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.code().as_deref() == Some("23505") => {
            AppError::Conflict("email already registered".into())
        }
        _ => e.into(),
    })
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"SELECT id, email, password_hash, quota_llm_tokens_monthly, quota_scrapes_daily FROM users WHERE email = $1"#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_user_by_id(pool: &PgPool, user_id: Uuid) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, password_hash, quota_llm_tokens_monthly, quota_scrapes_daily
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_user_seed(pool: &PgPool, user: &UserRow) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO users (
          id, email, password_hash, quota_llm_tokens_monthly, quota_scrapes_daily
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (id) DO UPDATE SET
          email = EXCLUDED.email,
          password_hash = EXCLUDED.password_hash,
          quota_llm_tokens_monthly = EXCLUDED.quota_llm_tokens_monthly,
          quota_scrapes_daily = EXCLUDED.quota_scrapes_daily
        "#,
    )
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(user.quota_llm_tokens_monthly)
    .bind(user.quota_scrapes_daily)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn resolve_api_key(pool: &PgPool, key_hash: &str) -> AppResult<AuthUser> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT u.id, u.email
        FROM api_keys k
        JOIN users u ON u.id = k.user_id
        WHERE k.key_hash = $1
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;
    let Some((user_id, email)) = row else {
        return Err(AppError::Unauthorized("invalid API key".into()));
    };
    sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE key_hash = $1")
        .bind(key_hash)
        .execute(pool)
        .await?;
    Ok(AuthUser { user_id, email })
}

pub async fn insert_api_key(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    key_hash: &str,
    key_hint: &str,
) -> AppResult<Uuid> {
    let id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO api_keys (user_id, name, key_hash, key_hint)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(key_hash)
    .bind(key_hint)
    .fetch_one(pool)
    .await?;
    Ok(id.0)
}

pub async fn list_api_keys(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<Vec<(Uuid, String, String, chrono::DateTime<Utc>)>> {
    sqlx::query_as(
        r#"
        SELECT id, name, key_hint, created_at
        FROM api_keys
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn delete_api_key(pool: &PgPool, user_id: Uuid, key_id: Uuid) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
        .bind(key_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// LLM tokens used in the current UTC calendar month (prompt + completion).
pub async fn llm_tokens_used_month(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    let month_start = NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
        .unwrap_or_else(|| Utc::now().date_naive());
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(llm_prompt_tokens + llm_completion_tokens), 0)::bigint
        FROM usage_daily
        WHERE user_id = $1 AND day >= $2
        "#,
    )
    .bind(user_id)
    .bind(month_start)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or(0))
}

pub async fn scrapes_used_today(pool: &PgPool, user_id: Uuid) -> AppResult<i32> {
    let today = Utc::now().date_naive();
    let row: Option<(i32,)> =
        sqlx::query_as(r#"SELECT scrapes FROM usage_daily WHERE user_id = $1 AND day = $2"#)
            .bind(user_id)
            .bind(today)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0).unwrap_or(0))
}

pub async fn ensure_llm_quota(pool: &PgPool, user_id: Uuid, planned_extra: i64) -> AppResult<()> {
    let u: (i64, i32) = sqlx::query_as(
        r#"SELECT quota_llm_tokens_monthly, quota_scrapes_daily FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("user not found".into()))?;
    let used = llm_tokens_used_month(pool, user_id).await?;
    if used + planned_extra > u.0 {
        return Err(AppError::Forbidden(format!(
            "LLM token quota exceeded ({used}/{})",
            u.0
        )));
    }
    Ok(())
}

pub async fn reserve_llm_tokens(
    pool: &PgPool,
    user_id: Uuid,
    planned_extra: i64,
) -> AppResult<i64> {
    if planned_extra <= 0 {
        return Ok(0);
    }
    let today = Utc::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
        .unwrap_or_else(|| Utc::now().date_naive());
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;
    let quota: (i64,) =
        sqlx::query_as(r#"SELECT quota_llm_tokens_monthly FROM users WHERE id = $1"#)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::Unauthorized("user not found".into()))?;
    let used: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(llm_prompt_tokens + llm_completion_tokens), 0)::bigint
        FROM usage_daily
        WHERE user_id = $1 AND day >= $2
        "#,
    )
    .bind(user_id)
    .bind(month_start)
    .fetch_one(&mut *tx)
    .await?;
    if used.0 + planned_extra > quota.0 {
        return Err(AppError::Forbidden(format!(
            "LLM token quota exceeded ({}/{})",
            used.0, quota.0
        )));
    }
    sqlx::query(
        r#"
        INSERT INTO usage_daily (user_id, day, llm_prompt_tokens, llm_completion_tokens, scrapes)
        VALUES ($1, $2, $3, 0, 0)
        ON CONFLICT (user_id, day) DO UPDATE SET
          llm_prompt_tokens = usage_daily.llm_prompt_tokens + EXCLUDED.llm_prompt_tokens
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(planned_extra)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(planned_extra)
}

pub async fn refund_llm_reservation(
    pool: &PgPool,
    user_id: Uuid,
    reserved_tokens: i64,
) -> AppResult<()> {
    if reserved_tokens <= 0 {
        return Ok(());
    }
    let today = Utc::now().date_naive();
    sqlx::query(
        r#"
        UPDATE usage_daily
        SET llm_prompt_tokens = GREATEST(llm_prompt_tokens - $3, 0)
        WHERE user_id = $1 AND day = $2
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(reserved_tokens)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reconcile_llm_reservation(
    pool: &PgPool,
    user_id: Uuid,
    reserved_tokens: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> AppResult<()> {
    if reserved_tokens <= 0 {
        return record_llm_usage(pool, user_id, prompt_tokens, completion_tokens).await;
    }
    if prompt_tokens + completion_tokens <= 0 {
        return Ok(());
    }
    let today = Utc::now().date_naive();
    sqlx::query(
        r#"
        UPDATE usage_daily
        SET
          llm_prompt_tokens = GREATEST(llm_prompt_tokens - $3, 0) + $4,
          llm_completion_tokens = llm_completion_tokens + $5
        WHERE user_id = $1 AND day = $2
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(reserved_tokens)
    .bind(prompt_tokens.max(0))
    .bind(completion_tokens.max(0))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn ensure_scrape_quota(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    let u: (i32,) = sqlx::query_as(r#"SELECT quota_scrapes_daily FROM users WHERE id = $1"#)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user not found".into()))?;
    let used = scrapes_used_today(pool, user_id).await?;
    if used >= u.0 {
        return Err(AppError::Forbidden(format!(
            "daily scrape quota exceeded ({}/{})",
            used, u.0
        )));
    }
    Ok(())
}

pub async fn reserve_scrapes(pool: &PgPool, user_id: Uuid, count: i32) -> AppResult<()> {
    if count <= 0 {
        return Ok(());
    }
    let today = Utc::now().date_naive();
    let mut tx = pool.begin().await?;
    let quota: (i32,) = sqlx::query_as(r#"SELECT quota_scrapes_daily FROM users WHERE id = $1"#)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user not found".into()))?;
    sqlx::query(
        r#"
        INSERT INTO usage_daily (user_id, day, llm_prompt_tokens, llm_completion_tokens, scrapes)
        VALUES ($1, $2, 0, 0, 0)
        ON CONFLICT (user_id, day) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(today)
    .execute(&mut *tx)
    .await?;
    let used: (i32,) = sqlx::query_as(
        r#"
        SELECT scrapes FROM usage_daily
        WHERE user_id = $1 AND day = $2
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .bind(today)
    .fetch_one(&mut *tx)
    .await?;
    if used.0 + count > quota.0 {
        return Err(AppError::Forbidden(format!(
            "daily scrape quota exceeded ({}/{})",
            used.0, quota.0
        )));
    }
    sqlx::query(
        r#"
        UPDATE usage_daily
        SET scrapes = scrapes + $3
        WHERE user_id = $1 AND day = $2
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(count)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn refund_scrapes(pool: &PgPool, user_id: Uuid, count: i32) -> AppResult<()> {
    if count <= 0 {
        return Ok(());
    }
    let today = Utc::now().date_naive();
    sqlx::query(
        r#"
        UPDATE usage_daily
        SET scrapes = GREATEST(scrapes - $3, 0)
        WHERE user_id = $1 AND day = $2
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(count)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_llm_usage(
    pool: &PgPool,
    user_id: Uuid,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> AppResult<()> {
    let today = Utc::now().date_naive();
    sqlx::query(
        r#"
        INSERT INTO usage_daily (user_id, day, llm_prompt_tokens, llm_completion_tokens, scrapes)
        VALUES ($1, $2, $3, $4, 0)
        ON CONFLICT (user_id, day) DO UPDATE SET
          llm_prompt_tokens = usage_daily.llm_prompt_tokens + EXCLUDED.llm_prompt_tokens,
          llm_completion_tokens = usage_daily.llm_completion_tokens + EXCLUDED.llm_completion_tokens
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_scrape(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    let today = Utc::now().date_naive();
    sqlx::query(
        r#"
        INSERT INTO usage_daily (user_id, day, llm_prompt_tokens, llm_completion_tokens, scrapes)
        VALUES ($1, $2, 0, 0, 1)
        ON CONFLICT (user_id, day) DO UPDATE SET
          scrapes = usage_daily.scrapes + 1
        "#,
    )
    .bind(user_id)
    .bind(today)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_agent_run(
    pool: &PgPool,
    user_id: Uuid,
    design_id: Option<Uuid>,
    status: &str,
    iterations: i32,
    total_prompt: i64,
    total_completion: i64,
    scrapes_used: i32,
    log: Value,
    summary: Option<&str>,
) -> AppResult<Uuid> {
    let id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO agent_runs (
          user_id, design_id, status, iterations,
          total_prompt_tokens, total_completion_tokens, scrapes_used, log, result_summary
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(design_id)
    .bind(status)
    .bind(iterations)
    .bind(total_prompt)
    .bind(total_completion)
    .bind(scrapes_used)
    .bind(log)
    .bind(summary)
    .fetch_one(pool)
    .await?;
    Ok(id.0)
}

pub fn empty_agent_log() -> Value {
    json!([])
}

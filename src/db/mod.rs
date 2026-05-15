//! Embedded PostgreSQL via pg-embed (local data dir, no Docker).

mod embedded;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::PathBuf;

pub use embedded::EmbeddedPostgres;

/// Start embedded Postgres, connect, run migrations, return pool (hold `EmbeddedPostgres` for process lifetime).
pub async fn connect(cfg: &crate::config::Config) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let data_dir = default_data_dir();
    let embedded = EmbeddedPostgres::start(&data_dir, cfg.embedded_port).await?;
    let url = embedded.database_url();
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&url)
        .await
        .with_context(|| format!("connect embedded postgres at {url}"))?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("run migrations")?;
    Ok((pool, embedded))
}

pub fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tokito")
        .join("postgres")
}

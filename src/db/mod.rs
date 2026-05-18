//! Embedded PostgreSQL via pg-embed (local data dir, no Docker).

mod embedded;
mod pg_embed_util;
pub mod pg_backup;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::PathBuf;

pub use embedded::EmbeddedPostgres;

/// Start embedded Postgres, connect, run migrations, return pool (hold `EmbeddedPostgres` for process lifetime).
pub async fn connect(cfg: &crate::config::Config) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let settings = crate::settings::load_file();
    let data_dir = crate::settings::postgres_data_dir(&settings);
    connect_embedded_at(&data_dir, cfg).await
}

/// Per-project embedded Postgres (`{workspace}/.data/postgres`).
pub async fn connect_project_embedded(
    workspace: &std::path::Path,
    cfg: &crate::config::Config,
) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let data_dir = crate::project_toml::ProjectToml::embedded_data_dir(workspace);
    connect_embedded_at(&data_dir, cfg).await
}

async fn connect_embedded_at(
    data_dir: &std::path::Path,
    cfg: &crate::config::Config,
) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let embedded =
        EmbeddedPostgres::start(data_dir, cfg.embedded_port, cfg.pg_embed_version).await?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseStatus {
    Starting,
    Ready,
    Degraded,
    Error,
}

pub async fn test_connection(pool: &PgPool) -> DatabaseStatus {
    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => DatabaseStatus::Ready,
        Err(_) => DatabaseStatus::Error,
    }
}

pub fn repair_cluster_dir(data_dir: &std::path::Path) -> anyhow::Result<()> {
    embedded::reset_cluster_dir(data_dir)
}

pub fn default_data_dir() -> PathBuf {
    crate::paths::default_postgres_data_dir()
}

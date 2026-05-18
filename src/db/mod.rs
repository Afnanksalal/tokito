//! Embedded PostgreSQL via pg-embed (local data dir, no Docker).

mod embedded;
pub mod pg_backup;
mod pg_embed_util;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::PathBuf;

pub use embedded::EmbeddedPostgres;

/// Start embedded Postgres, connect, run migrations, return pool (hold `EmbeddedPostgres` for process lifetime).
pub async fn connect(cfg: &crate::config::Config) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    connect_embedded_at(&cfg.postgres_data_dir, cfg).await
}

/// Per-project embedded Postgres (`{workspace}/.data/postgres`).
pub async fn connect_project_embedded(
    workspace: &std::path::Path,
    cfg: &crate::config::Config,
) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let data_dir = crate::project_toml::ProjectToml::embedded_data_dir(workspace);
    let port = available_project_port(cfg.embedded_port)
        .context("find available port for project PostgreSQL")?;
    connect_embedded_at_port(&data_dir, cfg, port).await
}

async fn connect_embedded_at(
    data_dir: &std::path::Path,
    cfg: &crate::config::Config,
) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    connect_embedded_at_port(data_dir, cfg, cfg.embedded_port).await
}

async fn connect_embedded_at_port(
    data_dir: &std::path::Path,
    cfg: &crate::config::Config,
    port: u16,
) -> anyhow::Result<(PgPool, EmbeddedPostgres)> {
    let embedded = EmbeddedPostgres::start(data_dir, port, cfg.pg_embed_version).await?;
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

fn available_project_port(global_port: u16) -> anyhow::Result<u16> {
    for candidate in ((global_port as u32 + 1)..=65_535).chain(10_240..global_port as u32) {
        let Ok(port) = u16::try_from(candidate) else {
            continue;
        };
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    anyhow::bail!("no local TCP port available for project PostgreSQL")
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

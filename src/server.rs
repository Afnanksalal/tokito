//! Embedded HTTP server (API + optional bundled SPA directory).

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;

pub async fn serve(bind: SocketAddr, spa_static_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let cfg = crate::config::load().context("load Tokito config")?;
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await
        .context("connect database")?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = crate::router::AppState::try_new(pool, &cfg)?;
    let app = crate::router::build(state, cfg.cors_origins.clone(), spa_static_dir);

    tracing::info!(%bind, "Tokito listening");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    axum::serve(listener, app).await?;
    Ok(())
}

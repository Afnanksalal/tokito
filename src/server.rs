//! Embedded HTTP server (API + optional bundled SPA directory).

use anyhow::Context;
use std::path::PathBuf;

pub async fn serve(
    cfg: crate::config::Config,
    spa_static_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bind: std::net::SocketAddr = cfg.http_addr.parse().context("parse http_addr")?;
    let (pool, embedded_pg) = crate::db::connect(&cfg).await.context("database")?;
    let _embedded_pg = embedded_pg;

    let state = crate::router::AppState::try_new(pool, &cfg)?;
    let app = crate::router::build(state, cfg.cors_origins.clone(), spa_static_dir);

    tracing::info!(%bind, "Tokito listening");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    axum::serve(listener, app).await?;
    Ok(())
}

use std::net::SocketAddr;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tokito=info,tower_http=info".into()),
        )
        .init();

    let spa_static_dir = std::env::var("TOKITO_STATIC_DIR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    if spa_static_dir.is_none() {
        tracing::info!(
            "Starting optional network listener (no static UI). Set TOKITO_STATIC_DIR to serve a web UI from disk."
        );
    }

    let cfg = tokito::config::load()?;
    let bind: SocketAddr = cfg.http_addr.parse()?;
    tokito::server::serve(bind, spa_static_dir).await
}

use std::net::SocketAddr;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
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
            "API-only mode (no static UI). Run `cargo run -p tokito-native` for the egui app, or set TOKITO_STATIC_DIR to serve an optional SPA from disk."
        );
    }

    let cfg = tokito::config::load()?;
    let bind: SocketAddr = cfg.http_addr.parse()?;
    tokito::server::serve(bind, spa_static_dir).await
}

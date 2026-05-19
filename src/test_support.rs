//! Shared helpers for integration tests (`test-support` feature).
//! Gating happens in `lib.rs` via `#[cfg(feature = "test-support")] pub mod test_support;`.

use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use std::future::Future;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::db;
use crate::router::{self, AppState};

static EMBEDDED: std::sync::OnceLock<Mutex<Option<db::EmbeddedPostgres>>> =
    std::sync::OnceLock::new();

/// HTTP integration tests that start embedded PostgreSQL (pg-embed downloads and extracts binaries).
///
/// Set **`TOKITO_RUN_DB_INTEGRATION=1`** to run them locally. CI sets this for the Linux job.
pub fn database_integration_tests_enabled() -> bool {
    matches!(
        std::env::var("TOKITO_RUN_DB_INTEGRATION")
            .ok()
            .as_deref()
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// Postgres pool for integration tests (embedded pg-embed under the temp dir).
pub async fn test_pool() -> anyhow::Result<PgPool> {
    let lock = EMBEDDED.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().await;
    if guard.is_none() {
        let dir = std::env::temp_dir().join(format!("tokito_test_pg_{}", std::process::id()));
        let port = std::env::var("TOKITO_TEST_EMBEDDED_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(pick_ephemeral_port);
        let pg = tokio::time::timeout(
            Duration::from_secs(300),
            db::EmbeddedPostgres::start(&dir, port, 16),
        )
        .await
        .context("embedded postgres setup timed out")??;
        *guard = Some(pg);
    }
    let url = guard.as_ref().expect("embedded").database_url();
    drop(guard);

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Bearer token for a fresh test user (unique email per call — avoids cross-test
/// pollution where an assertion like `list.len() == 1` would otherwise race when
/// every test shares the same identity in the embedded Postgres.
pub async fn test_bearer(pool: &PgPool, jwt_secret: &str) -> anyhow::Result<String> {
    let email = format!("it-{}@tokito.local", uuid::Uuid::new_v4().simple());
    test_bearer_for(pool, jwt_secret, &email).await
}

/// Like [`test_bearer`] but with an explicit email — use when a test needs a
/// stable identity (e.g. asserting the same user across multiple requests).
pub async fn test_bearer_for(
    pool: &PgPool,
    jwt_secret: &str,
    email: &str,
) -> anyhow::Result<String> {
    use crate::auth::encode_session_jwt;
    use crate::store::account;

    let user = match account::find_user_by_email(pool, email).await? {
        Some(u) => u,
        None => {
            let hash = bcrypt::hash("test-password-8chars", 4)?;
            account::create_user(pool, email, &hash, Some("Test")).await?
        }
    };
    let token = encode_session_jwt(user.id, &user.email, jwt_secret)?;
    Ok(format!("Bearer {token}"))
}

/// Run an async integration test with a generous timeout (embedded Postgres first start can be slow).
pub async fn with_timeout<F, T>(fut: F) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>>,
{
    tokio::time::timeout(Duration::from_secs(300), fut)
        .await
        .context("integration test timed out after 300s")?
}

/// Axum router backed by [`test_pool`].
pub async fn test_router() -> anyhow::Result<(Router, String)> {
    let pool = test_pool().await?;
    let cfg = crate::config::Config::for_tests();
    let bearer = test_bearer(&pool, &cfg.jwt_secret).await?;
    let state = AppState::try_new(pool, &cfg)?;
    Ok((router::build(state, vec![], None), bearer))
}

fn pick_ephemeral_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .map(|l| l.local_addr().expect("bind").port())
        .unwrap_or(17_334)
}

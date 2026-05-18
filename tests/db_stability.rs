//! Embedded Postgres stability (§7). Requires `TOKITO_RUN_DB_INTEGRATION=1`.

use sqlx::PgPool;
use std::net::TcpListener;
use std::path::PathBuf;
use tokito::db::EmbeddedPostgres;

fn enabled() -> bool {
    tokito::test_support::database_integration_tests_enabled()
}

async fn pool_for(url: &str) -> anyhow::Result<PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[tokio::test]
async fn postgres_data_survives_sequential_start() -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }
    let dir = std::env::temp_dir().join(format!(
        "tokito_pg_seq_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir)?;
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

    let mut pg1 = EmbeddedPostgres::start(&dir, port, 16).await?;
    let pool = pool_for(&pg1.database_url()).await?;
    let marker = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO designs (id, name) VALUES ($1, $2)")
        .bind(marker)
        .bind("stability-probe")
        .execute(&pool)
        .await?;
    pool.close().await;
    pg1.graceful_stop().await;
    drop(pg1);

    let _pg2 = EmbeddedPostgres::start(&dir, port, 16).await?;
    let pool2 = pool_for(&_pg2.database_url()).await?;
    let row: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM designs WHERE id = $1")
            .bind(marker)
            .fetch_optional(&pool2)
            .await?;
    assert!(row.is_some());
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[tokio::test]
async fn port_conflict_does_not_wipe_cluster() -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }
    let dir = std::env::temp_dir().join(format!(
        "tokito_pg_port_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

    let mut pg = EmbeddedPostgres::start(&dir, port, 16).await?;
    pg.graceful_stop().await;
    drop(pg);

    let version_file: PathBuf = dir.join("PG_VERSION");
    assert!(version_file.is_file());

    let _blocker = TcpListener::bind(("127.0.0.1", port))?;
    let err = EmbeddedPostgres::start(&dir, port, 16).await;
    assert!(err.is_err());
    assert!(version_file.is_file());
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

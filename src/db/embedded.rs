//! Embedded PostgreSQL via [pg-embed](https://docs.rs/pg-embed) — Tokito's only database backend.

use anyhow::Context;
use pg_embed::pg_access::PgAccess;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::Error as PgEmbedError;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V16, PG_V17, PG_V18};
use pg_embed::postgres::{PgEmbed, PgSettings};
use std::path::Path;
use std::time::Duration;

const DB_NAME: &str = "tokito";
const USER: &str = "tokito";
const PASSWORD: &str = "tokito";
const PG_VERSION_FILE: &str = "PG_VERSION";
const START_ATTEMPTS: u32 = 5;

pub struct EmbeddedPostgres {
    pg: PgEmbed,
}

impl EmbeddedPostgres {
    pub async fn start(data_dir: &Path, port: u16) -> anyhow::Result<Self> {
        prepare_cluster_dir(data_dir)?;

        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..START_ATTEMPTS {
            match try_start(data_dir, port).await {
                Ok(pg) => return Ok(pg),
                Err(e) => {
                    if should_purge_pg_embed_cache(&e) {
                        tracing::warn!(
                            error = %e,
                            attempt = attempt + 1,
                            "pg-embed binary cache or download failed — purging OS cache pg-embed folder and retrying",
                        );
                        if let Err(purge_err) = PgAccess::purge().await {
                            tracing::warn!(%purge_err, "pg-embed cache purge failed");
                        }
                    } else {
                        tracing::warn!(
                            error = %e,
                            attempt = attempt + 1,
                            "embedded postgres failed — resetting data dir and retrying",
                        );
                    }
                    reset_cluster_dir(data_dir)?;
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.expect("embedded postgres retry loop should record last error"))
            .context("embedded PostgreSQL failed after retries")
    }

    pub fn database_url(&self) -> String {
        self.pg.full_db_uri(DB_NAME)
    }
}

impl Drop for EmbeddedPostgres {
    fn drop(&mut self) {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            let _ = rt.block_on(self.pg.stop_db());
        }
    }
}

fn should_purge_pg_embed_cache(err: &anyhow::Error) -> bool {
    if err
        .chain()
        .find_map(|c| c.downcast_ref::<PgEmbedError>())
        .is_some_and(|e| {
            matches!(
                e,
                PgEmbedError::UnpackFailure
                    | PgEmbedError::InvalidPgPackage
                    | PgEmbedError::InvalidPgUrl
                    | PgEmbedError::DownloadFailure(_)
                    | PgEmbedError::ConversionFailure(_)
                    | PgEmbedError::WriteFileError(_)
            )
        })
    {
        return true;
    }
    let msg = err.to_string();
    msg.contains("Failed to unpack PostgreSQL binaries.")
        || msg.contains("Invalid PostgreSQL binaries package.")
        || msg.contains("Invalid PostgreSQL binaries download URL.")
        || msg.contains("PostgreSQL binaries download failure:")
        || msg.contains("Request response bytes conversion failure:")
        || msg.contains("Could not read file:")
        || msg.contains("Could not write to file:")
}

/// Postgres version for pg-embed Maven artifacts. Set `TOKITO_PG_EMBED_VERSION` to `16`, `17`, or `18`.
/// Default **16** — often fewer Windows unpack issues than newer bundles.
fn pg_fetch_settings() -> PgFetchSettings {
    let raw = std::env::var("TOKITO_PG_EMBED_VERSION")
        .ok()
        .map(|s| s.trim().to_lowercase());
    let version = match raw.as_deref() {
        Some("18") => PG_V18,
        Some("17") => PG_V17,
        Some("16") | None => PG_V16,
        Some(other) => {
            tracing::warn!(
                other,
                "TOKITO_PG_EMBED_VERSION invalid (use 16, 17, or 18); using 16"
            );
            PG_V16
        }
    };
    PgFetchSettings {
        version,
        ..Default::default()
    }
}

/// Remove a half-initialized cluster so `initdb` can run (non-empty dir without `PG_VERSION` fails).
fn prepare_cluster_dir(data_dir: &Path) -> anyhow::Result<()> {
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).context("create embedded data dir")?;
        return Ok(());
    }
    let version = data_dir.join(PG_VERSION_FILE);
    if version.is_file() {
        return Ok(());
    }
    reset_cluster_dir(data_dir)
}

fn reset_cluster_dir(data_dir: &Path) -> anyhow::Result<()> {
    if data_dir.exists() {
        std::fs::remove_dir_all(data_dir).context("remove postgres data dir")?;
    }
    std::fs::create_dir_all(data_dir).context("create embedded data dir")?;
    Ok(())
}

async fn try_start(data_dir: &Path, port: u16) -> anyhow::Result<EmbeddedPostgres> {
    let settings = PgSettings {
        database_dir: data_dir.to_path_buf(),
        port,
        user: USER.into(),
        password: PASSWORD.into(),
        auth_method: PgAuthMethod::Plain,
        persistent: true,
        timeout: Some(Duration::from_secs(180)),
        migration_dir: None,
    };
    let fetch = pg_fetch_settings();

    let mut pg = PgEmbed::new(settings, fetch)
        .await
        .context("init pg-embed")?;
    tokio::time::timeout(Duration::from_secs(300), pg.setup())
        .await
        .context("pg-embed setup timed out (check network for first-time binary download)")?
        .context("pg-embed setup")?;
    tokio::time::timeout(Duration::from_secs(120), pg.start_db())
        .await
        .context("embedded postgres start timed out")?
        .context("start embedded postgres")?;

    if !pg.database_exists(DB_NAME).await.context("check db")? {
        pg.create_database(DB_NAME)
            .await
            .context("create tokito database")?;
    }

    Ok(EmbeddedPostgres { pg })
}

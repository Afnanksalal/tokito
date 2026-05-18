//! Embedded PostgreSQL via [pg-embed](https://docs.rs/pg-embed).

use anyhow::Context;
use pg_embed::pg_access::PgAccess;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::Error as PgEmbedError;
use pg_embed::postgres::{PgEmbed, PgSettings};

use crate::db::pg_embed_util::{pg_executables_ready, pg_fetch_settings};
use std::path::Path;
use std::time::Duration;

const DB_NAME: &str = "tokito";
const USER: &str = "tokito";
const PASSWORD: &str = "tokito";
const PG_VERSION_FILE: &str = "PG_VERSION";
const START_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartFailureKind {
    /// Wipe cluster data dir and retry (failed initdb, version mismatch, broken cluster).
    CorruptCluster,
    /// Purge pg-embed binary cache and re-download (missing/corrupt binaries).
    CacheOrDownload,
    /// Port busy, lock files, etc. — do not wipe data.
    Transient,
}

pub struct EmbeddedPostgres {
    pg: PgEmbed,
}

impl EmbeddedPostgres {
    pub async fn start(data_dir: &Path, port: u16, pg_version: u16) -> anyhow::Result<Self> {
        prepare_cluster_dir(data_dir, pg_version)?;

        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..START_ATTEMPTS {
            if let Err(e) = ensure_pg_embed_binaries(data_dir, pg_version).await {
                tracing::warn!(error = %e, attempt = attempt + 1, "pg-embed binaries not ready");
                if let Err(purge_err) = PgAccess::purge().await {
                    tracing::warn!(%purge_err, "pg-embed cache purge failed");
                }
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1)))
                    .await;
                continue;
            }

            match try_start(data_dir, port, pg_version).await {
                Ok(pg) => return Ok(pg),
                Err(e) => {
                    let kind = classify_start_error(&e, data_dir, pg_version);
                    match kind {
                        StartFailureKind::CacheOrDownload => {
                            tracing::warn!(
                                error = %e,
                                attempt = attempt + 1,
                                "pg-embed cache/download failed — purging cache and retrying",
                            );
                            if let Err(purge_err) = PgAccess::purge().await {
                                tracing::warn!(%purge_err, "pg-embed cache purge failed");
                            }
                        }
                        StartFailureKind::CorruptCluster => {
                            tracing::warn!(
                                error = %e,
                                attempt = attempt + 1,
                                "postgres data dir unusable — resetting cluster",
                            );
                            reset_cluster_dir(data_dir)?;
                        }
                        StartFailureKind::Transient => {
                            tracing::warn!(
                                error = %e,
                                attempt = attempt + 1,
                                "embedded postgres transient failure — retrying",
                            );
                            tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1)))
                                .await;
                        }
                    }
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

    pub async fn graceful_stop(&mut self) {
        let _ = self.pg.stop_db().await;
    }
}

/// Download/unpack PostgreSQL binaries before `setup()` so an existing cluster is not re-inited.
async fn ensure_pg_embed_binaries(data_dir: &Path, pg_version: u16) -> anyhow::Result<()> {
    let fetch = pg_fetch_settings(pg_version);
    let access = PgAccess::new(&fetch, data_dir)
        .await
        .context("pg-embed access init")?;
    if pg_executables_ready(&access).await.context("check pg binaries")? {
        return Ok(());
    }
    tracing::info!(
        version = pg_version,
        cache = %access.cache_dir.display(),
        "downloading embedded PostgreSQL binaries (first run may take a few minutes)",
    );
    tokio::time::timeout(Duration::from_secs(300), access.maybe_acquire_postgres())
        .await
        .context("pg-embed binary download timed out")?
        .context("pg-embed binary download")?;
    crate::db::pg_embed_util::repair_windows_pg_embed_cache(&access.cache_dir)
        .context("repair pg-embed windows binary paths")?;
    if !pg_executables_ready(&access).await.context("recheck pg binaries")? {
        anyhow::bail!(
            "pg-embed binaries missing after download (expected initdb under {}/bin)",
            access.cache_dir.display()
        );
    }
    Ok(())
}

fn classify_start_error(
    err: &anyhow::Error,
    data_dir: &Path,
    requested_version: u16,
) -> StartFailureKind {
    if should_purge_pg_embed_cache(err) {
        return StartFailureKind::CacheOrDownload;
    }
    if is_pg_init_failure(err) {
        if cluster_major_version(data_dir).is_some() {
            // Existing cluster + initdb ran → almost always missing/wrong binary cache.
            return StartFailureKind::CacheOrDownload;
        }
        return StartFailureKind::CorruptCluster;
    }
    if cluster_needs_reset(data_dir, requested_version) {
        return StartFailureKind::CorruptCluster;
    }
    if is_port_conflict(err) {
        return StartFailureKind::Transient;
    }
    if is_pg_start_failure(err) && cluster_needs_reset(data_dir, requested_version) {
        return StartFailureKind::CorruptCluster;
    }
    StartFailureKind::Transient
}

fn is_pg_init_failure(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|c| c.downcast_ref::<PgEmbedError>() == Some(&PgEmbedError::PgInitFailure))
        || err.to_string().contains("PostgreSQL could not be initialized")
}

fn is_pg_start_failure(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|c| c.downcast_ref::<PgEmbedError>() == Some(&PgEmbedError::PgStartFailure))
        || err.to_string().contains("PostgreSQL could not be started")
}

fn is_port_conflict(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("address already in use")
        || msg.contains("could not bind")
        || msg.contains("is another postmaster")
}

fn cluster_major_version(data_dir: &Path) -> Option<u16> {
    let raw = std::fs::read_to_string(data_dir.join(PG_VERSION_FILE)).ok()?;
    raw.trim().parse().ok()
}

fn cluster_needs_reset(data_dir: &Path, requested_version: u16) -> bool {
    if cluster_looks_corrupt(data_dir) {
        return true;
    }
    match cluster_major_version(data_dir) {
        Some(major) if major != requested_version => true,
        _ => false,
    }
}

fn cluster_looks_corrupt(data_dir: &Path) -> bool {
    if !data_dir.exists() {
        return false;
    }
    if data_dir.join(PG_VERSION_FILE).is_file() {
        return false;
    }
    std::fs::read_dir(data_dir)
        .ok()
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
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
        || msg.contains("pg-embed binary download")
        || msg.contains("pg-embed binaries missing after download")
}

fn prepare_cluster_dir(data_dir: &Path, requested_version: u16) -> anyhow::Result<()> {
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).context("create embedded data dir")?;
        return Ok(());
    }
    if cluster_needs_reset(data_dir, requested_version) {
        tracing::warn!(
            dir = %data_dir.display(),
            requested = requested_version,
            found = ?cluster_major_version(data_dir),
            "resetting postgres cluster (version mismatch or incomplete data)",
        );
        reset_cluster_dir(data_dir)?;
    }
    Ok(())
}

pub fn reset_cluster_dir(data_dir: &Path) -> anyhow::Result<()> {
    remove_pwfile(data_dir);
    if data_dir.exists() {
        std::fs::remove_dir_all(data_dir).context("remove postgres data dir")?;
    }
    std::fs::create_dir_all(data_dir).context("create embedded data dir")?;
    Ok(())
}

fn remove_pwfile(data_dir: &Path) {
    let mut pw = data_dir.to_path_buf();
    pw.set_extension("pwfile");
    let _ = std::fs::remove_file(pw);
}

async fn try_start(data_dir: &Path, port: u16, pg_version: u16) -> anyhow::Result<EmbeddedPostgres> {
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
    let fetch = pg_fetch_settings(pg_version);

    let mut pg = PgEmbed::new(settings, fetch)
        .await
        .context("init pg-embed")?;
    tokio::time::timeout(Duration::from_secs(120), pg.setup())
        .await
        .context("pg-embed setup timed out")?
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn valid_cluster_with_pg_version_is_not_corrupt() {
        let dir = std::env::temp_dir().join(format!("tokito_pg_ok_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(PG_VERSION_FILE), "16\n").unwrap();
        assert!(!cluster_looks_corrupt(&dir));
        assert!(!cluster_needs_reset(&dir, 16));
        assert!(cluster_needs_reset(&dir, 17));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn init_failure_with_existing_cluster_is_cache_not_wipe() {
        let dir = std::env::temp_dir().join(format!("tokito_pg_init_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(PG_VERSION_FILE), "16\n").unwrap();
        let err = anyhow::anyhow!(PgEmbedError::PgInitFailure).context("pg-embed setup");
        assert_eq!(
            classify_start_error(&err, Path::new(&dir), 16),
            StartFailureKind::CacheOrDownload
        );
        assert!(dir.join(PG_VERSION_FILE).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn port_in_use_classified_transient() {
        let dir = std::env::temp_dir().join(format!("tokito_pg_port_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(PG_VERSION_FILE), "16\n").unwrap();
        let err = anyhow::anyhow!("could not bind: Address already in use");
        assert_eq!(
            classify_start_error(&err, Path::new(&dir), 16),
            StartFailureKind::Transient
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

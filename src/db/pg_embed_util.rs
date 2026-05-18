//! Work around pg-embed 1.0 checking extensionless binary paths on Windows.

use std::path::Path;

use pg_embed::pg_access::PgAccess;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V16, PG_V17, PG_V18};

pub fn pg_fetch_settings(version: u16) -> PgFetchSettings {
    let pg = match version {
        18 => PG_V18,
        17 => PG_V17,
        _ => PG_V16,
    };
    PgFetchSettings {
        version: pg,
        ..Default::default()
    }
}

/// True when `bin/initdb` and `bin/pg_ctl` exist (pg-embed's layout).
pub fn pg_embed_binaries_present(cache_dir: &Path) -> bool {
    let bin = cache_dir.join("bin");
    required_pg_tool(&bin, "initdb").is_file() && required_pg_tool(&bin, "pg_ctl").is_file()
}

pub fn required_pg_tool(bin_dir: &Path, name: &str) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        let exe = bin_dir.join(format!("{name}.exe"));
        if exe.is_file() {
            return exe;
        }
    }
    bin_dir.join(name)
}

/// pg-embed's `pg_executables_cached` uses `bin/initdb`; Windows ships `initdb.exe`.
#[cfg(windows)]
pub fn repair_windows_pg_embed_cache(cache_dir: &Path) -> std::io::Result<()> {
    let bin = cache_dir.join("bin");
    if !bin.is_dir() {
        return Ok(());
    }
    for tool in ["initdb", "pg_ctl", "postgres", "pg_dump", "psql"] {
        let plain = bin.join(tool);
        let exe = bin.join(format!("{tool}.exe"));
        if exe.is_file() && !plain.is_file() && std::fs::hard_link(&exe, &plain).is_err() {
            std::fs::copy(&exe, &plain)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn repair_windows_pg_embed_cache(_cache_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

pub async fn pg_executables_ready(access: &PgAccess) -> anyhow::Result<bool> {
    if access.pg_executables_cached().await? {
        return Ok(true);
    }
    repair_windows_pg_embed_cache(&access.cache_dir)?;
    Ok(access.pg_executables_cached().await?)
}

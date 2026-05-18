//! Cluster backup via `pg_dump` (embedded cache binaries, then PATH).

use std::path::{Path, PathBuf};
use std::process::Command;

use pg_embed::pg_access::PgAccess;
use url::Url;

use crate::db::pg_embed_util::{pg_embed_binaries_present, pg_fetch_settings, required_pg_tool};

/// Run `pg_dump` into `dest_sql` (best-effort; writes stub SQL if `pg_dump` missing).
pub async fn pg_dump_to_file(
    database_url: &str,
    dest_sql: &Path,
    pg_embed_version: u16,
) -> anyhow::Result<()> {
    let embedded = resolve_embedded_pg_dump(pg_embed_version).await;
    let url = database_url.to_string();
    let dest = dest_sql.to_path_buf();
    tokio::task::spawn_blocking(move || run_pg_dump(&url, &dest, embedded.as_deref()))
        .await
        .map_err(|e| anyhow::anyhow!("pg_dump task: {e}"))?
}

async fn resolve_embedded_pg_dump(version: u16) -> Option<PathBuf> {
    let fetch = pg_fetch_settings(version);
    let probe_dir = std::env::temp_dir().join("tokito-pg-embed-probe");
    let access = PgAccess::new(&fetch, &probe_dir).await.ok()?;
    if access.maybe_acquire_postgres().await.is_err() {
        return None;
    }
    let _ = crate::db::pg_embed_util::repair_windows_pg_embed_cache(&access.cache_dir);
    if !pg_embed_binaries_present(&access.cache_dir) {
        return None;
    }
    let path = required_pg_tool(&access.cache_dir.join("bin"), "pg_dump");
    path.is_file().then_some(path)
}

fn run_pg_dump(
    database_url: &str,
    dest_sql: &Path,
    preferred: Option<&Path>,
) -> anyhow::Result<()> {
    if let Some(exe) = preferred {
        if try_pg_dump(exe, database_url, dest_sql).is_ok() {
            return Ok(());
        }
    }
    let candidates: &[&str] = if cfg!(windows) {
        &["pg_dump.exe", "pg_dump"]
    } else {
        &["pg_dump"]
    };
    for exe in candidates {
        if try_pg_dump(Path::new(exe), database_url, dest_sql).is_ok() {
            return Ok(());
        }
    }
    std::fs::write(
        dest_sql,
        "-- Tokito: pg_dump not available; design JSON exports in this archive remain valid.\n",
    )?;
    Ok(())
}

fn try_pg_dump(exe: &Path, database_url: &str, dest_sql: &Path) -> anyhow::Result<()> {
    let conn = PgDumpConnection::parse(database_url);
    let mut cmd = Command::new(exe);
    conn.apply(&mut cmd);
    let out = cmd
        .arg("--no-owner")
        .arg("--no-acl")
        .arg("-f")
        .arg(dest_sql)
        .output()?;
    if out.status.success() {
        return Ok(());
    }
    tracing::warn!(
        exe = %exe.display(),
        stderr = %String::from_utf8_lossy(&out.stderr),
        "pg_dump failed"
    );
    anyhow::bail!("pg_dump exit {}", out.status);
}

struct PgDumpConnection {
    host: Option<String>,
    port: Option<String>,
    user: Option<String>,
    password: Option<String>,
    database: String,
    sslmode: Option<String>,
}

impl PgDumpConnection {
    fn parse(database_url: &str) -> Self {
        let Ok(url) = Url::parse(database_url) else {
            return Self {
                host: None,
                port: None,
                user: None,
                password: None,
                database: database_url.to_string(),
                sslmode: None,
            };
        };
        let database = url.path().trim_start_matches('/').to_string();
        let sslmode = url
            .query_pairs()
            .find(|(k, _)| k == "sslmode")
            .map(|(_, v)| v.to_string());
        Self {
            host: url.host_str().map(str::to_string),
            port: url.port().map(|p| p.to_string()),
            user: (!url.username().is_empty()).then(|| url.username().to_string()),
            password: url.password().map(str::to_string),
            database: if database.is_empty() {
                "postgres".to_string()
            } else {
                database
            },
            sslmode,
        }
    }

    fn apply(&self, cmd: &mut Command) {
        if let Some(host) = &self.host {
            cmd.env("PGHOST", host);
        }
        if let Some(port) = &self.port {
            cmd.env("PGPORT", port);
        }
        if let Some(user) = &self.user {
            cmd.env("PGUSER", user);
        }
        if let Some(password) = &self.password {
            cmd.env("PGPASSWORD", password);
        }
        if let Some(sslmode) = &self.sslmode {
            cmd.env("PGSSLMODE", sslmode);
        }
        cmd.arg("--dbname").arg(&self.database);
    }
}

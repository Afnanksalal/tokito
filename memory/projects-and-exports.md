# Projects, workspaces & exports

Projects are both DB rows and workspace folders. `projects.workspace_path` points at the folder; new projects default under `%LOCALAPPDATA%\tokito\projects\<slug>` via `src/paths.rs`, with a `project.toml` written by `src/store/projects.rs`.

`project.toml` (`src/project_toml.rs`) stores `id`, `name`, `slug`, `database.mode`, and `exports.default_format`. `database.mode = "global"` is the default. `database.mode = "embedded"` is an opt-in per-project Postgres cluster under `<workspace>/.data/postgres`; the Studio currently exposes this as text in Settings / project UI, not as a global settings toggle.

Design exports go under `<workspace>/exports` unless the user chooses another path in the save dialog. Supported native/HTTP formats are SVG, PDF plot, PDF pack, connectivity netlist `.txt`, Tokito S-expression netlist `.net`, BOM CSV, MCAD handoff JSON, and bundle ZIP.

Backups go under `<workspace>/backups/<design>_<timestamp>` and include the design export bundle plus a best-effort `database.sql` when `pg_dump` is available. `src/db/pg_backup.rs` parses the DB URL into `PGHOST`/`PGPORT`/`PGUSER`/`PGPASSWORD`/`--dbname` instead of putting the full URL on the process command line.

Project ZIP import/export (`src/services/project_archive.rs`) writes a `project_manifest.json`, per-design folders with `schematic_document.json` and exports, optional `project.toml`, and optional `database.sql`. Import uses `ZipArchive::enclosed_name()` and caps archives at 2,000 entries / 512 MiB extracted.

**How to apply:**

- Keep generated exports/backups inside the project workspace unless the user explicitly picks a different path.
- When adding import/export behavior, update both native Studio and shared `export_service` / `project_archive` paths so HTTP tests and desktop stay aligned.
- Be careful when changing project DB behavior: code may be operating on the global pool or an opt-in per-project embedded pool depending on `project.toml`.

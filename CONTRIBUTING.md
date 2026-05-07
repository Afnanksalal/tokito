# Contributing to Tokito

Thank you for helping improve Tokito. This project is a Rust workspace (`tokito` library + API binary, `tokito-native` desktop app).

## Getting started

1. Install **Rust** (stable, **1.74+**) and **PostgreSQL** (**14+**, CI uses 16).
2. Copy `.env.example` to `.env` and point `TOKITO_DATABASE_URL` at a local database.
3. Start Postgres (e.g. `docker compose up -d postgres` if you use the bundled Compose file).
4. Run checks locally:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

5. Optional integration test (separate DB recommended):

   ```bash
   export TOKITO_TEST_DATABASE_URL="postgres://tokito:tokito@localhost:5433/tokito_test?sslmode=disable"
   cargo test -p tokito --test integration -- --ignored --nocapture
   ```

   PowerShell:

   ```powershell
   $env:TOKITO_TEST_DATABASE_URL = "postgres://..."
   cargo test -p tokito --test integration -- --ignored --nocapture
   ```

## Pull requests

- Keep commits focused; match existing style (`cargo fmt`, naming, module layout).
- Update **docs** (`README.md`, `docs/API.md`) when behavior or env vars change.
- Do not commit secrets (`.env`, API keys). `.env.example` is the place for documented placeholders.

## Security

Please report sensitive issues per [`SECURITY.md`](SECURITY.md), not public issues.

## License

Unless you state otherwise, contributions are accepted under the same terms as the project: **MIT OR Apache-2.0** (SPDX), at the recipient’s option. See `LICENSE-MIT` and `LICENSE-APACHE`.

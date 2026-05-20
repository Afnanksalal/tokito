# Testing & CI

**Local pre-PR check** (per CONTRIBUTING.md and CI):

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace          # or: cargo nextest run --workspace
```

## Integration tests

Integration tests live in a **single custom-harness binary** at `tests/integration/main.rs` — each topic is a submodule (`mod api_designs;`, `mod golden_document;`, …). This is deliberate: the embedded Postgres cluster (`OnceLock<EmbeddedPostgres>` in `src/test_support.rs`) starts **once per `cargo test` run**, not once per file as it would with separate `tests/*.rs` binaries.

Run them with `cargo test -p tokito --test integration` or `make test-db`.

**DB integration tests** are gated by **`TOKITO_RUN_DB_INTEGRATION=1`** because they spin up embedded Postgres (first run downloads binaries). That env var is the **only** switch — the old `GITHUB_ACTIONS` auto-enable was removed. `make test-db` sets it.

**Adding a test:** create (or extend) a submodule file under `tests/integration/` and add a `mod` line to `tests/integration/main.rs`. No `Cargo.toml` change needed — there is no per-file enumeration anymore.

## Golden / snapshot tests

Golden tests use **`insta`** snapshots, stored in `tests/integration/snapshots/`. CI sets `INSTA_UPDATE=no` so any drift fails the build; locally run `cargo insta review` to inspect and accept changes.

## Test support

The `tokito` lib exposes a `test-support` feature; `dev-dependencies` include `tokito = { path = ".", features = ["test-support"] }`, so helpers (`Config::for_tests`, `test_router`, `test_bearer`, fixtures in `src/test_support.rs`) are gated behind it. `test_bearer` mints a **fresh uuid-suffixed user per call** to avoid cross-test pollution.

## CI

`.github/workflows/ci.yml` (overhauled 2026-05-20):

- **`test` job** — matrix of `ubuntu-latest` + `windows-latest`. DB integration runs **Linux-only** (`TOKITO_RUN_DB_INTEGRATION` set per-OS via matrix expression; pg-embed on Windows is slow/fragile). Steps: `cargo check` → fmt → clippy → `cargo nextest run` → doctests.
- **`coverage` job** — `cargo-llvm-cov` + nextest → Codecov (non-gating; needs the `CODECOV_TOKEN` secret for PR comments).
- **`deny` job** — `cargo-deny` (advisories + licenses + bans + sources; config in `deny.toml`).
- Test runner is **`cargo-nextest`** with profile `ci` from `.config/nextest.toml` (retries, slow-timeout, JUnit).
- All GitHub Actions are **SHA-pinned** with `# version` comments. Concurrency cancels in-progress runs on the same ref.
- There is **no Dependabot** — the config was intentionally removed.

`.github/workflows/release.yml` builds + packages on `windows-latest` for `v*` tags.

**How to apply:** keep code lint-clean on both Rust 1.88 and current stable (see [architecture.md](architecture.md)). When the Windows `install-action` step flakes with a "bash startup failure", that's a known transient runner issue — re-run the job.

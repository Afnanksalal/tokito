# Contributing to Tokito

Thanks for improving Tokito. This workspace ships:

- **`tokito`** — library + **`cargo run -p tokito`** HTTP API  
- **`tokito-native`** — **`cargo run -p tokito-native`** desktop (egui)

Both share migrations, domain logic, and integrations.

---

## Before you open a PR

From the repo root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Optional Postgres integration test (create a test DB and set the URL):

```bash
export TOKITO_TEST_DATABASE_URL="postgres://tokito:tokito@localhost:5433/tokito_test?sslmode=disable"
cargo test -p tokito --test integration -- --ignored --nocapture
```

PowerShell:

```powershell
$env:TOKITO_TEST_DATABASE_URL = "postgres://..."
cargo test -p tokito --test integration -- --ignored --nocapture
```

---

## Guidelines

- **Small, focused commits** with messages that explain *why*, not only *what*.
- **Match existing style** — modules, naming, error handling (`AppError`), SQLx patterns.
- **Update docs** when behavior or env vars change — especially **`README.md`** and **`docs/API.md`**.
- **Never commit secrets** — use **`.env.example`** for new configuration knobs only.

---

## Security

Do **not** file undisclosed vulnerabilities as public GitHub issues. Follow **[SECURITY.md](SECURITY.md)**.

---

## Licensing

By submitting a contribution, you agree it may be distributed under the project’s terms: **MIT**. See [`LICENSE-MIT`](LICENSE-MIT).

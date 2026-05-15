# Contributing to Tokito

Thanks for improving Tokito. This workspace ships:

- **`tokito`** — library + **`cargo run -p tokito`** HTTP API  
- **`tokito-native`** — **`cargo run -p tokito-native`** desktop (egui)

Both share migrations, domain logic, and integrations.

```mermaid
flowchart TB
  subgraph workspace[Workspace]
    C[tokito crate — lib + HTTP binary]
    N[tokito-native — desktop binary]
  end
  subgraph shared[Shared]
    M[migrations/]
    T[tests / SQLx]
  end
  C --- M
  N --- M
  C --- T
```

---

## Before you open a PR

```mermaid
flowchart LR
  F["cargo fmt --check"] --> L["cargo clippy<br/>-D warnings"]
  L --> U["cargo test --workspace"]
  U --> I{"DB-related<br/>change?"}
  I -->|optional deep check| D["TOKITO_RUN_DB_INTEGRATION=1<br/>api_* tests"]
  I --> PR[Open PR]
  D --> PR
```

From the repo root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

API integration tests (embedded Postgres; set **`TOKITO_RUN_DB_INTEGRATION=1`**, first run may download binaries). CI sets this automatically.

```bash
TOKITO_RUN_DB_INTEGRATION=1 cargo test -p tokito --test api_designs --test api_parts --test api_schematic
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

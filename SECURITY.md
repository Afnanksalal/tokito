# Security policy

## Supported releases

Security fixes land on the **default branch** (`main` / `master`). Tokito **0.1.x** does not have separate long-term support branches yet.

---

## Reporting a vulnerability

**Please do not** open a **public** issue for an undisclosed security bug.

1. Use GitHub **private vulnerability reporting** if it is enabled for this repository (**Security → Report a vulnerability**).
2. Otherwise, contact the maintainers privately with a subject line such as `Security: Tokito <short summary>`.

Include:

- **Affected surface** — HTTP API, native app, auth, integrations, etc.
- **Version or commit** you tested.
- **Steps to reproduce** or a minimal proof of concept, if it can be shared safely.

We aim to acknowledge valid reports within a **few business days** and coordinate disclosure after a fix is available.

---

## Scope & expectations

- Tokito stores **design and catalog data** and may call **third-party APIs** (xAI, Firecrawl, distributors). **Operational security matters**: protect **`TOKITO_JWT_SECRET`**, database credentials, and API keys; use **TLS** in production; restrict network access to Postgres.
- **Supply chain**: run **`cargo audit`** or enable **Dependabot** in your fork/org as part of your own review process.

Thank you for helping keep users safe.

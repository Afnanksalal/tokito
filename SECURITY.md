# Security policy

## Supported versions

Security fixes are applied to the default branch (`main` or `master`) going forward. There are no separate long-term support releases yet for Tokito **0.1.x**.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for undisclosed security vulnerabilities.

- Prefer **private vulnerability reporting** via GitHub (“Security” → “Report a vulnerability”) if enabled for this repository.
- Otherwise, contact the maintainers with a clear subject line (e.g. “Security: Tokito …”) and enough detail to reproduce or assess impact.

Include: affected component (API, native app, auth), version/commit, and steps or proof-of-concept if safe to share.

We aim to acknowledge reasonable reports within a few business days and coordinate disclosure after a fix is available.

## Scope notes

- Tokito stores **design data** and may integrate **third-party APIs** (xAI, Firecrawl, distributors). Misconfiguration (exposed `.env`, weak `TOKITO_JWT_SECRET`) is an operational risk—rotate secrets and use TLS in production.
- Dependency advisories: run `cargo audit` (or GitHub Dependabot) as part of your own supply-chain review.

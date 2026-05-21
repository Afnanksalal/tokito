# AGENTS.md

**For AI coding assistants working in this repo.** Read this file at the start of every session before doing anything else, then read the memory files relevant to your task and check the project board (see [The board](#the-board)).

This is not user-facing documentation — humans have `README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, and `docs/`. This file plus the `memory/` folder are the AI's shared notebook: facts one of us learned that all of us should know. The repo is the sync mechanism — `git pull` gets you the team's latest knowledge, `git push` (or a PR) gives the team yours.

---

## How this works

- **`AGENTS.md`** (this file) — the *instructions*: how to read and maintain the notebook, plus the index below.
- **`memory/`** — the *facts*: one Markdown file per topic. Project-shaped knowledge lives here, not inline in this file.

## The board

Active work — roadmap items, features, bugs, tasks — is tracked on the **GitHub Projects board**: <https://github.com/orgs/VtronTokito/projects/1>. Every card is a GitHub issue. This is the live "what are we doing and where does it stand" view; `ROADMAP.md` stays the human-facing horizon.

Afnan and Joel both work from this board, and **the agents are responsible for keeping it honest** — a card's state should never lag the real state of the work. This is not optional housekeeping; it is how the two of us stay in sync.

**At session start**, after reading this file, run `scripts/board.sh ls` to see what's in flight. If your task matches an existing card, work against it instead of starting an untracked thread.

**As you work**, update the board in the same rhythm as your commits:

| When | Command |
|---|---|
| Taking on work that has no card | `scripts/board.sh new "<title>" --type <bug\|feature\|task> --owner <Afnan\|Joel>` |
| Starting work on a card | `scripts/board.sh move <issue#> "In progress"` |
| Meaningful progress or a decision worth recording | `scripts/board.sh note <issue#> "<what changed>"` |
| Work done, PR open | `scripts/board.sh move <issue#> "In review"` |
| Merged / shipped | `scripts/board.sh done <issue#>` |

Every update is attributed automatically to the GitHub account you are authenticated as — Joel is `kakarot-dev`, Afnan is his own account — so the *who* is recorded for free. Set the **Owner** field (`Afnan` / `Joel` / `Both`) to say who is driving a card.

Full command reference, field definitions, raw `gh` fallbacks, and the one-time web setup live in [memory/board-workflow.md](memory/board-workflow.md).

## How to use the memory files

**Reading.** Read this file first, every session. Then read the `memory/` files relevant to what you're about to do — don't read all of them blindly. Treat their contents as authoritative for project-shaped facts; they override anything stale in your own per-project agent memory.

**Updating.** Edit the relevant `memory/` file in-place during normal work, in the same commit as the code change that made a fact true or false. Each file uses a `**Why:**` / `**How to apply:**` style where the reasoning isn't self-evident — mirror it.

**Adding a fact.** You learned something non-obvious and verified it: a quirky build flag, a constraint, the *why* behind a decision, external context that affects the work. Put it in the most relevant existing `memory/` file. Only create a new file for a genuinely new topic — and add it to the index below in the same commit.

**Deleting.** A section is fully obsolete (feature removed, decision reversed) — delete it. No tombstones or `# DEPRECATED` blocks; git history is the record.

**Verify before recommending.** A line that names a specific file, function, flag, or version is a claim that was true *when written*. If you're about to act on it (not just describe history), grep / read first and update if it drifted.

## What does NOT belong in `memory/`

- Things `README.md` / `CONTRIBUTING.md` / `docs/` already explain. Update those instead and reference them.
- Things obvious from skimming the code (file structure, naming, what a function does).
- Ephemeral state (today's task, current branch, in-progress TODOs) or your own agent preferences — those go in your local agent memory, not the repo.
- Secrets, tokens, internal URLs.

## Commit hygiene

Memory edits go in their own commit unless they're part of a code change that made the fact true — then bundle them so the diffs read together. Commit message style: `memory: <file> — <what changed>`.

---

## Memory index

| File | What's in it |
|---|---|
| [memory/product.md](memory/product.md) | Product framing — desktop AI-assisted schematic studio; "AI proposes, you approve"; local-first. |
| [memory/architecture.md](memory/architecture.md) | Workspace shape, the two crates, key tech/versions and toolchain to expect before opening files. |
| [memory/data-model.md](memory/data-model.md) | Postgres schema — tables, triggers, how designs are structured; migration rules. |
| [memory/settings-and-providers.md](memory/settings-and-providers.md) | `settings.toml` (the primary config) and the supported AI providers. |
| [memory/env-vars.md](memory/env-vars.md) | Every `TOKITO_*` / runtime env var, what it overlays, and what is deliberately *not* an env var. |
| [memory/http-api.md](memory/http-api.md) | The optional `/v1` Axum surface — what it is and is not. |
| [memory/ui-design-language.md](memory/ui-design-language.md) | Native egui studio — stack, dock layout, design tokens, and egui 0.29 idioms & footguns. |
| [memory/projects-and-exports.md](memory/projects-and-exports.md) | Project workspace folders, `project.toml`, exports, backups, and project zip import/export. |
| [memory/testing-and-ci.md](memory/testing-and-ci.md) | How to run tests, the single integration harness, snapshot tests, and the CI pipeline. |
| [memory/docs-reference.md](memory/docs-reference.md) | Where the canonical human docs and scripts live. |
| [memory/env-linux-wslg.md](memory/env-linux-wslg.md) | Env-specific — running the desktop binary on WSL2/WSLg. Ignore unless that's your setup. |
| [memory/board-workflow.md](memory/board-workflow.md) | The GitHub Projects board — what it tracks, the `scripts/board.sh` commands agents update it with, and one-time web setup. |

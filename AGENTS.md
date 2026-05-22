# AGENTS.md

**For AI coding assistants working in this repo.** Read this file at the start of every session before doing anything else, then read the memory files relevant to your task and check the project board (see [The board](#the-board)).

This is not user-facing documentation — humans have `README.md`, `CONTRIBUTING.md`, and `docs/`. This file plus the `memory/` folder are the AI's shared notebook: facts one of us learned that all of us should know. The repo is the sync mechanism — `git pull` gets you the team's latest knowledge, `git push` (or a PR) gives the team yours.

---

## How this works

- **`AGENTS.md`** (this file) — the *instructions*: how to read and maintain the notebook, plus the index below.
- **`memory/`** — the *facts*: one Markdown file per topic. Project-shaped knowledge lives here, not inline in this file.

## The board

Active work — roadmap items, features, bugs, tasks — is tracked on the **GitHub Projects board**: <https://github.com/orgs/VtronTokito/projects/1>. Every card is a GitHub issue. **The board *is* the roadmap** — there is no separate roadmap file; the `roadmap`-labelled cards are the product horizon.

Afnan and Joel both work from this board, and **the agents are responsible for keeping it honest** — a card's state should never lag the real state of the work. This is not optional housekeeping; it is how the two of us stay in sync.

### At session start

After reading this file, run `scripts/board.sh ls` to see what's in flight. If your task matches an existing card, work against it instead of starting an untracked thread.

### Classifying work — what goes on the board

Everything trackable is an issue with exactly one **type label**. Choose it by what the work *is*:

| Label | Use it for | Example |
|---|---|---|
| `roadmap` | A horizon-level product direction — broad, spans many releases. Collectively, these *are* the roadmap. | "PCB layout: routing and copper" |
| `epic` | A large body of work that breaks into several issues. A `roadmap` card usually spawns `epic`s once started. | "Footprint linkage subsystem" |
| `feature` | A concrete, shippable capability or enhancement. | "Snap wires to nearest pin" |
| `bug` | Shipped behaviour is wrong or broken. Reactive — a bug is **not** roadmap. | "Wire detaches on symbol rotate" |
| `task` | A concrete unit of work, usually a child of an `epic` or `feature`. | "Add ERC rule for floating nets" |
| `chore` | Maintenance — tooling, deps, CI, infra. Not user-facing. | "Bump egui to 0.30" |

The shape is a tree: **`roadmap` → `epic` → `feature` / `task`**. `bug` and `chore` are orthogonal — they come from shipped code, not from the roadmap. Rule of thumb for `feature` vs `roadmap`: if a user could try it within one release, it's a `feature`; if it's a direction made of many releases, it's `roadmap`.

Bugs and roadmap items live on the **same** board — they are separated by *view*, not by separate boards. The **Roadmap** view filters to `roadmap`/`epic`; the **Board** (kanban) view shows everything. So: never skip the board for a bug, and never expect a bug to clutter the roadmap.

### Keep the board updated as you work

| When | Command |
|---|---|
| Taking on work that has no card | `scripts/board.sh new "<title>" --type <label> --owner <Afnan\|Joel>` |
| Starting work on a card | `scripts/board.sh move <issue#> "In progress"` |
| Meaningful progress or a decision worth recording | `scripts/board.sh note <issue#> "<what changed>"` |
| Work done, PR open | `scripts/board.sh move <issue#> "In review"` |
| Merged / shipped | `scripts/board.sh done <issue#>` |

### Link PRs and commits to the board

A card is only useful if it connects to the work that resolves it. Always:

- **Every PR** must put a closing keyword in its description — `Closes #<issue#>` (use `Fixes #<issue#>` for bugs). GitHub then links the PR under the card's *linked pull requests* and moves the card to **Done** on merge.
- **Commit messages** should reference the issue (`#<issue#>`) so the commit appears on the card's timeline.
- **No untracked PRs.** If a change has no issue, it has no card — create the issue first (`scripts/board.sh new ...`), then open the PR against it.

Every update is attributed automatically to the GitHub account you are authenticated as — Joel is `kakarot-dev`, Afnan is his own account — so the *who* is recorded for free. Set the **Owner** field (`Afnan` / `Joel` / `Both`) to say who is driving a card.

Full command reference, field definitions, raw `gh` fallbacks, and the one-time web setup live in [memory/board-workflow.md](memory/board-workflow.md).

## How to use the memory files

**Reading.** Read this file first, every session. Then read the `memory/` files relevant to what you're about to do — don't read all of them blindly. Treat their contents as authoritative for project-shaped facts; they override anything stale in your own per-project agent memory.

> **⚠️ Task-gated reads — mandatory, not "relevant if you feel like it".** Some work has a memory file you MUST read *before writing any code*. Reading `native/` / `src/` source is **not** a substitute — source shows what exists, not the rules, and much of `native/` is pre-`tokito_ui` legacy you must not extend.
>
> | Before you… | Read first, in full |
> |---|---|
> | write or change **any UI / egui / frontend code** | [memory/ui-design-language.md](memory/ui-design-language.md) |
>
> The UI file carries a **STRICT, review-blocking rule**: every UI primitive (button, input, dialog, chip, row, toggle, …) is defined in the shared **`tokito_ui`** library (repo `github.com/VtronTokito/ui`) and only *composed* in this app — **never** hand-rolled in `native/src/` with raw `egui` `Frame`/`Button`/`Area`/painter calls. Need a new primitive, or a tweak to one? Change it in `tokito_ui` first, push, `cargo update -p tokito_ui`, then consume it.

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
| [memory/ui-design-language.md](memory/ui-design-language.md) | **Mandatory pre-read for any UI work.** The STRICT `tokito_ui`-only component rule — UI primitives are never hand-rolled in `native/`. Plus the egui stack, dock layout, design tokens, and egui 0.29 footguns. |
| [memory/projects-and-exports.md](memory/projects-and-exports.md) | Project workspace folders, `project.toml`, exports, backups, and project zip import/export. |
| [memory/testing-and-ci.md](memory/testing-and-ci.md) | How to run tests, the single integration harness, snapshot tests, and the CI pipeline. |
| [memory/docs-reference.md](memory/docs-reference.md) | Where the canonical human docs and scripts live. |
| [memory/env-linux-wslg.md](memory/env-linux-wslg.md) | Env-specific — running the desktop binary on WSL2/WSLg. Ignore unless that's your setup. |
| [memory/board-workflow.md](memory/board-workflow.md) | The GitHub Projects board — what it tracks, the `scripts/board.sh` commands agents update it with, and one-time web setup. |

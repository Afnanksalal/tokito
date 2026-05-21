# Board workflow (GitHub Projects + `scripts/board.sh`)

The project board is the live tracker for everything in motion — roadmap items, features, bugs, tasks. `ROADMAP.md` is the human-facing horizon; the board is "what are we doing and where does it stand."

- **Board:** <https://github.com/orgs/VtronTokito/projects/1> (org `VtronTokito`, project number `1`)
- **Every card is a GitHub issue** in `VtronTokito/tokito`. Nothing is a draft issue — real issues carry labels, comments, and PR links.
- **Who-stamp:** every issue edit and comment is attributed to the GitHub account that made it — Joel is `kakarot-dev`, Afnan is his own account. No manual signing needed.

**Why a Projects board, not a file:** it is the only no-build option that gives a real kanban + a who-stamp + cross-linking to PRs and commits. The tradeoff — it lives outside `git pull` — is accepted; agents reach it through `gh` / `scripts/board.sh`, not their file context.

## Fields

| Field | Values | Meaning |
|---|---|---|
| **Status** | Aim · Next up · In progress · In review · Done | Where the work stands. The board columns. |
| **Owner** | Afnan · Joel · Both · Unassigned | Who is driving the card (distinct from GitHub assignee). |
| **Area** | Schematic & Library · PCB Layout · AI & Automation · Production · Platform | Mirrors the `ROADMAP.md` horizon sections. |
| **Priority** | P0 · P1 · P2 | P0 = drop-everything. |

**Type is a label, not a field** — `bug`, `feature`, `epic`, `roadmap`, `task`, `chore`. One issue, classified by label, keeps the Issues tab and the board in agreement. `epic` is for a large item that spawns child issues.

## `scripts/board.sh` — the everyday interface

Resolves project/field/option IDs live, so it survives field edits. Run from the repo root.

```
board.sh ls                              # list cards grouped by status
board.sh new "<title>" --type bug|feature|epic|task|chore \
                       [--area "<area>"] [--owner Afnan|Joel|Both] \
                       [--priority P0|P1|P2] [--status "<status>"] [--body "<text>"]
board.sh set  <issue#> <Status|Owner|Area|Priority> <value>
board.sh move <issue#> <status>          # shortcut for: set Status
board.sh note <issue#> "<text>"          # post a progress comment (auto-tagged as agent)
board.sh done <issue#>                   # Status=Done + close the issue
board.sh web                             # open the board in a browser
```

Agents must keep cards in step with reality — see the **The board** section of `AGENTS.md` for when to call which command.

## Raw `gh` fallback

`board.sh` covers the common path. For anything else:

- IDs — Project: `PVT_kwDOEQxcRM4BYWnU`. Field IDs: Status `PVTSSF_lADOEQxcRM4BYWnUzhTdEZM`, Owner `PVTSSF_lADOEQxcRM4BYWnUzhTdEs4`, Area `PVTSSF_lADOEQxcRM4BYWnUzhTdEtw`, Priority `PVTSSF_lADOEQxcRM4BYWnUzhTdEus`.
- Option IDs drift if options are edited — fetch live: `gh project field-list 1 --owner VtronTokito --format json`.
- `gh project item-edit` sets **one field per call** and needs `--id <item-id> --project-id <id> --field-id <id> --single-select-option-id <id>`. This verbosity is exactly what `board.sh` hides.
- The token needs the `project` scope: `gh auth refresh -s project`. `repo` scope (already present) covers all `gh issue` use.

## One-time web setup

The CLI cannot create board views or workflows — do these once in the browser:

1. **Board view** — open the project → add a view → layout **Board**, group by **Status**. This is the kanban.
2. **Roadmap/Table views** — optionally add a view grouped by **Area** for a roadmap-style read, and a **Table** view for triage.
3. **Workflows** (project → ⋯ → Workflows) — enable so the board self-maintains:
   - *Item added to project* → set **Status** = `Aim`
   - *Item closed* → set **Status** = `Done`
   - *Auto-archive items* → when `Done` and closed
   With these on, agents rarely touch `Status` directly except `In progress` / `In review`.

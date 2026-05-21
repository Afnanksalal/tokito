#!/usr/bin/env bash
# Tokito project board helper — see memory/board-workflow.md.
#
# The board lives at https://github.com/orgs/VtronTokito/projects/1
# Every card is a GitHub issue. This script wraps the verbose
# `gh project item-edit` calls so agents and humans update the board
# with one readable command. Field/option IDs are resolved live, so it
# keeps working if fields or options change.
set -euo pipefail

OWNER="VtronTokito"
REPO="VtronTokito/tokito"
PROJECT="1"
BOARD_URL="https://github.com/orgs/VtronTokito/projects/1"

die() { echo "board: $*" >&2; exit 1; }
command -v gh      >/dev/null 2>&1 || die "missing dependency: gh"
command -v python3 >/dev/null 2>&1 || die "missing dependency: python3"

CACHE="$(mktemp -d "${TMPDIR:-/tmp}/tokito-board.XXXXXX")"
trap 'rm -rf "$CACHE"' EXIT

ensure_pid()    { [ -s "$CACHE/pid" ]    || gh project view "$PROJECT" --owner "$OWNER" --format json --jq .id >"$CACHE/pid"; }
ensure_fields() { [ -s "$CACHE/fields" ] || gh project field-list "$PROJECT" --owner "$OWNER" --format json >"$CACHE/fields"; }
ensure_items()  { [ -s "$CACHE/items" ]  || gh project item-list "$PROJECT" --owner "$OWNER" --limit 500 --format json >"$CACHE/items"; }
project_id()    { ensure_pid; cat "$CACHE/pid"; }

# resolve "<field name>" "<option name>" -> prints "<field-id> <option-id>"
resolve() {
  ensure_fields
  python3 - "$CACHE/fields" "$1" "$2" <<'PY'
import json, sys
path, fname, oname = sys.argv[1], sys.argv[2].lower(), sys.argv[3].lower()
with open(path) as fh:
    d = json.load(fh)
for f in d["fields"]:
    if f["name"].lower() == fname:
        for o in f.get("options", []):
            if o["name"].lower() == oname:
                print(f["id"], o["id"]); sys.exit(0)
        have = ", ".join(o["name"] for o in f.get("options", []))
        sys.exit(f"board: no option '{sys.argv[3]}' in field '{sys.argv[2]}' (have: {have})")
sys.exit(f"board: no field '{sys.argv[2]}'")
PY
}

# item_id <issue#> -> prints the project item id
item_id() {
  ensure_items
  python3 - "$CACHE/items" "$1" <<'PY'
import json, sys
path, num = sys.argv[1], int(sys.argv[2])
with open(path) as fh:
    d = json.load(fh)
for it in d["items"]:
    if (it.get("content") or {}).get("number") == num:
        print(it["id"]); sys.exit(0)
sys.exit(f"board: issue #{num} is not on the board")
PY
}

set_item() {  # <item-id> <field> <option>
  local pair fid oid
  pair="$(resolve "$2" "$3")"
  fid="${pair%% *}"; oid="${pair##* }"
  gh project item-edit --id "$1" --project-id "$(project_id)" \
    --field-id "$fid" --single-select-option-id "$oid" >/dev/null
}

set_field() {  # <issue#> <field> <option>
  set_item "$(item_id "$1")" "$2" "$3"
  echo "board: #$1  $2 -> $3"
}

cmd_new() {
  [ $# -ge 1 ] || die "usage: board.sh new \"<title>\" [--type T] [--area A] [--owner O] [--priority P] [--status S] [--body B]"
  local title="$1"; shift
  local type="" area="" owner="" priority="" status="Aim" body=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --type)     type="$2";     shift 2 ;;
      --area)     area="$2";     shift 2 ;;
      --owner)    owner="$2";    shift 2 ;;
      --priority) priority="$2"; shift 2 ;;
      --status)   status="$2";   shift 2 ;;
      --body)     body="$2";     shift 2 ;;
      *) die "unknown flag: $1" ;;
    esac
  done
  [ -n "$body" ] || body="Tracked on the [board]($BOARD_URL)."
  local args=(--repo "$REPO" --title "$title" --body "$body")
  [ -n "$type" ] && args+=(--label "$type")
  local url num iid
  url="$(gh issue create "${args[@]}")"
  num="${url##*/}"
  # Use the item id returned by item-add directly — re-querying item-list
  # right after can miss the new card due to replication lag.
  iid="$(gh project item-add "$PROJECT" --owner "$OWNER" --url "$url" --format json --jq .id)"
  set_item "$iid" Status "$status"
  [ -n "$area" ]     && set_item "$iid" Area "$area"
  [ -n "$owner" ]    && set_item "$iid" Owner "$owner"
  [ -n "$priority" ] && set_item "$iid" Priority "$priority"
  echo "board: created #$num  $url"
}

cmd_set() {
  [ $# -eq 3 ] || die "usage: board.sh set <issue#> <Status|Owner|Area|Priority> <value>"
  set_field "$1" "$2" "$3"
}

cmd_move() {
  [ $# -eq 2 ] || die "usage: board.sh move <issue#> <status>"
  set_field "$1" Status "$2"
}

cmd_done() {
  [ $# -eq 1 ] || die "usage: board.sh done <issue#>"
  set_field "$1" Status Done
  gh issue close "$1" --repo "$REPO" \
    --comment "Done — closed via board on $(date +%F). _(update by agent)_" >/dev/null
  echo "board: #$1 closed"
}

cmd_note() {
  [ $# -eq 2 ] || die "usage: board.sh note <issue#> \"<text>\""
  gh issue comment "$1" --repo "$REPO" --body "$2"$'\n\n_(update by agent)_' >/dev/null
  echo "board: noted on #$1"
}

cmd_ls() {
  ensure_items
  python3 - "$CACHE/items" <<'PY'
import json, sys
with open(sys.argv[1]) as fh:
    d = json.load(fh)
order = ["Aim", "Next up", "In progress", "In review", "Done"]
buckets = {}
for it in d["items"]:
    c = it.get("content") or {}
    if "number" not in c:
        continue
    buckets.setdefault(it.get("status", "—"), []).append(
        (c["number"], c.get("title", ""), it.get("owner", "")))
for s in order + [k for k in buckets if k not in order]:
    rows = buckets.get(s)
    if not rows:
        continue
    print(f"\n## {s}")
    for num, title, owner in sorted(rows):
        tag = f"  [{owner}]" if owner else ""
        print(f"  #{num}  {title}{tag}")
PY
}

cmd_web() { gh project view "$PROJECT" --owner "$OWNER" --web; }

usage() {
  cat <<EOF
board.sh — Tokito project board ($BOARD_URL)

  board.sh new "<title>" [--type bug|feature|epic|task|chore]
                         [--area "<area>"] [--owner Afnan|Joel|Both]
                         [--priority P0|P1|P2] [--status "<status>"] [--body "<text>"]
  board.sh set  <issue#> <Status|Owner|Area|Priority> <value>
  board.sh move <issue#> <status>          # shortcut for: set Status
  board.sh done <issue#>                   # Status=Done + close issue
  board.sh note <issue#> "<text>"          # post a progress comment
  board.sh ls                              # list cards grouped by status
  board.sh web                             # open the board in a browser

Status values: Aim, Next up, In progress, In review, Done
EOF
}

case "${1:-}" in
  new)  shift; cmd_new  "$@" ;;
  set)  shift; cmd_set  "$@" ;;
  move) shift; cmd_move "$@" ;;
  done) shift; cmd_done "$@" ;;
  note) shift; cmd_note "$@" ;;
  ls)   shift; cmd_ls   "$@" ;;
  web)  shift; cmd_web  "$@" ;;
  ""|-h|--help|help) usage ;;
  *) die "unknown command: $1 (try: board.sh --help)" ;;
esac

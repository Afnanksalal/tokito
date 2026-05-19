#!/usr/bin/env bash
# Build + run the Tokito desktop studio on Linux (or WSL2 with WSLg).
# Counterpart to scripts/package-windows.ps1.
#
# Usage:
#   scripts/run-linux.sh                # debug build + run
#   scripts/run-linux.sh --release      # release build + run
#   scripts/run-linux.sh --check        # cargo check only
#   scripts/run-linux.sh --package      # release build, stage into dist/Tokito/
#   scripts/run-linux.sh --no-deps      # skip apt dependency check

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="run"
PROFILE="debug"
CHECK_DEPS=1

for arg in "$@"; do
  case "$arg" in
    --release) PROFILE="release" ;;
    --check)   MODE="check" ;;
    --package) MODE="package"; PROFILE="release" ;;
    --no-deps) CHECK_DEPS=0 ;;
    -h|--help)
      sed -n '2,11p' "$0"; exit 0 ;;
    *)
      echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

# ----- system deps (mirrors .github/workflows/ci.yml) -------------------------

REQUIRED_PKGS=(
  libgtk-3-dev
  libx11-dev
  libxcb-render0-dev
  libxcb-shape0-dev
  libxcb-xfixes0-dev
  libxkbcommon-dev
  libwayland-dev
)

if [[ "$CHECK_DEPS" == 1 ]] && command -v dpkg-query >/dev/null 2>&1; then
  missing=()
  for pkg in "${REQUIRED_PKGS[@]}"; do
    if ! dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed"; then
      missing+=("$pkg")
    fi
  done
  if (( ${#missing[@]} > 0 )); then
    echo "Missing apt packages: ${missing[*]}"
    echo "Install with:"
    echo "  sudo apt-get install -y ${missing[*]}"
    exit 1
  fi
fi

# ----- toolchain --------------------------------------------------------------

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust 1.88+ (https://rustup.rs)." >&2
  exit 1
fi

# rust-toolchain.toml pins the version; rustup will auto-fetch.
cargo --version

# ----- WSL display sanity (informational, non-fatal) --------------------------

if grep -qiE "(microsoft|wsl)" /proc/version 2>/dev/null; then
  if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
    echo "Note: running under WSL with no DISPLAY/WAYLAND_DISPLAY set."
    echo "      WSLg (Win11) usually exports these automatically; otherwise run an X server."
  fi
fi

# ----- run --------------------------------------------------------------------

case "$MODE" in
  check)
    exec cargo check -p tokito-native --locked
    ;;
  run)
    if [[ "$PROFILE" == "release" ]]; then
      exec cargo run --release -p tokito-native
    else
      exec cargo run -p tokito-native
    fi
    ;;
  package)
    echo "Building release binary..."
    cargo build --release -p tokito-native

    OUT="$ROOT/dist/Tokito"
    rm -rf "$OUT"
    mkdir -p "$OUT"

    cp "$ROOT/target/release/tokito-native" "$OUT/Tokito"
    cp -r "$ROOT/assets" "$OUT/assets"

    cat > "$OUT/README.txt" <<'EOF'
Tokito - desktop schematic studio. Describe the board; AI drafts; you refine.

1. Run ./Tokito (keep the assets folder beside it).
2. Open Settings and add your AI provider key + Firecrawl API key.
3. First launch may prepare the local database (internet needed once).

Your designs: ~/.local/share/tokito/  (or XDG_DATA_HOME).
See docs/SETTINGS.md in the source repo for all keys.
EOF

    echo
    echo "Ready: $OUT"
    echo "Run:   $OUT/Tokito"
    ;;
esac

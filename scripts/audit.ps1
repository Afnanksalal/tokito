# Security and stability audit (fmt, clippy, tests, cargo-deny).
# CI runs the same set; this script is the local equivalent.
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host "=== cargo fmt check ==="
cargo fmt --all -- --check

Write-Host "=== cargo clippy (workspace) ==="
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | Out-Host

Write-Host "=== cargo test (unit) ==="
cargo test --workspace 2>&1 | Out-Host

Write-Host "=== cargo test (integration) ==="
$env:TOKITO_RUN_DB_INTEGRATION = "1"
cargo test -p tokito --test integration -- --nocapture 2>&1 | Out-Host

Write-Host "=== cargo deny (advisories, licenses, bans, sources) ==="
if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    cargo deny check 2>&1 | Out-Host
} else {
    Write-Warning "cargo-deny not installed; run: cargo install --locked cargo-deny"
}

Write-Host "=== Done ==="

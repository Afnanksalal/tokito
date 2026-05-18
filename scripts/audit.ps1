# Security and stability audit (fmt, clippy, tests, cargo-audit).
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host "=== cargo fmt check ==="
cargo fmt --all -- --check

Write-Host "=== cargo clippy (lib) ==="
cargo clippy -p tokito -- -D warnings 2>&1 | Out-Host

Write-Host "=== cargo test (unit) ==="
cargo test -p tokito -p tokito-native 2>&1 | Out-Host

Write-Host "=== cargo test (integration) ==="
$env:TOKITO_RUN_DB_INTEGRATION = "1"
cargo test -p tokito `
  --test api_designs --test api_parts --test api_schematic `
  --test golden_document --test golden_netlist_move `
  --test services_exports --test spec_compliance `
  --test db_stability --test notes_research --test project_workspace `
  --test ai_pipeline_fixtures 2>&1 | Out-Host

Write-Host "=== cargo audit (advisory DB) ==="
if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    cargo audit 2>&1 | Out-Host
} else {
    Write-Warning "cargo-audit not installed; run: cargo install cargo-audit"
}

Write-Host "=== Done ==="

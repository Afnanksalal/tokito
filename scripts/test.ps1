# Run workspace tests. Stops stale test binaries that block the linker on Windows (LNK1104).
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))

Get-Process -Name "tokito-native","Tokito" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Get-ChildItem "$PSScriptRoot\..\target\debug\deps\api_*.exe" -ErrorAction SilentlyContinue |
    ForEach-Object { Stop-Process -Name $_.BaseName -Force -ErrorAction SilentlyContinue }

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test --workspace -- --nocapture
exit $LASTEXITCODE

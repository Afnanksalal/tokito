# Build a portable Windows folder: double-click Tokito.exe (no shell required).
# Requires: Rust 1.88+, MSVC build tools.

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "Building release binary..."
cargo build --release -p tokito-native
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "dist\Tokito"
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
New-Item -ItemType Directory -Path $Out | Out-Null

$ExeSrc = Join-Path $Root "target\release\tokito-native.exe"
$ExeDst = Join-Path $Out "Tokito.exe"
Copy-Item $ExeSrc $ExeDst

$AssetsSrc = Join-Path $Root "assets"
$AssetsDst = Join-Path $Out "assets"
Copy-Item -Recurse $AssetsSrc $AssetsDst

@"
Tokito — desktop schematic studio. Describe the board; AI drafts; you refine.

1. Double-click Tokito.exe (keep the assets folder beside it)
2. Open Settings and add your xAI and Firecrawl API keys
3. First launch may prepare the local database (internet needed once)

Your designs: %LOCALAPPDATA%\tokito\
See docs/SETTINGS.md in the source repo for all keys.
"@ | Set-Content -Encoding UTF8 (Join-Path $Out "README.txt")

Write-Host ""
Write-Host "Ready: $Out"
Write-Host "Run: $ExeDst"

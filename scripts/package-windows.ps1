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

Copy-Item (Join-Path $Root ".env.example") (Join-Path $Out ".env.example")

@"
Tokito — AI builds the board; you edit the result

1. Copy .env.example to .env beside Tokito.exe
2. Set TOKITO_XAI_API_KEY and TOKITO_FIRECRAWL_API_KEY (required for Build)
3. Double-click Tokito.exe
4. First launch may download embedded PostgreSQL (needs internet once)

Data: %LOCALAPPDATA%\tokito\
"@ | Set-Content -Encoding UTF8 (Join-Path $Out "README.txt")

Write-Host ""
Write-Host "Ready: $Out"
Write-Host "Run: $ExeDst"

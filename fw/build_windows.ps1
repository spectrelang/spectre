$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
    throw "clang is required to build the Windows bootstrap compiler"
}

$TempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
if (-not $TempRoot) {
    $TempRoot = Join-Path $Root ".tmp"
}
$Hash = "unknown"
try {
    $Hash = (git rev-parse --short HEAD).Trim()
} catch {
}

Write-Host "Building Windows bootstrap compiler..."
clang -O3 -Wno-everything -Wno-error=int-conversion `
    bootstrap/sxcw.c `
    -lDbghelp `
    -lbcrypt `
    -o stage0.exe
if (-not (Test-Path "stage0.exe")) {
    throw "bootstrap compiler build failed"
}

Write-Host "Building spectre stage 1..."
./stage0.exe ./src/sxc.sx -release -alt -runtime:hosted -o ./spectre-stage1.exe
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "spectre-stage1.exe")) {
    throw "stage 1 build failed (exit $LASTEXITCODE)"
}

Write-Host "Building spectre-dev..."
./spectre-stage1.exe ./src/sxc.sx -release -alt -runtime:hosted -o ./spectre-dev.exe -d:GIT_HASH=$Hash
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "spectre-dev.exe")) {
    throw "stage 2 build failed (exit $LASTEXITCODE)"
}

Write-Host "Windows compiler build complete."

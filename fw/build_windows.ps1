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
$YyjsonDir = if ($env:YYJSON_DIR) { $env:YYJSON_DIR } else { Join-Path $TempRoot "yyjson" }
$YyjsonSrc = Join-Path $YyjsonDir "src"

if (-not (Test-Path (Join-Path $YyjsonSrc "yyjson.c"))) {
    if (-not (Test-Path $YyjsonDir)) {
        git clone --depth 1 https://github.com/ibireme/yyjson.git $YyjsonDir
    }
}
if (-not (Test-Path (Join-Path $YyjsonSrc "yyjson.c"))) {
    throw "could not find yyjson.c under $YyjsonDir"
}

$Hash = "unknown"
try {
    $Hash = (git rev-parse --short HEAD).Trim()
} catch {
}

Write-Host "Building runtime objects..."
clang -O2 -Wno-error=int-conversion -c spectrelib/csources/panic_handler.c -o spectrelib/csources/panic_handler.o
clang -O2 -Wno-error=int-conversion -I $YyjsonSrc -c spectrelib/csources/yyjson_shim.c -o spectrelib/csources/yyjson_shim.o

Write-Host "Building Windows bootstrap compiler..."
clang -O3 -Wno-everything -Wno-error=int-conversion `
    bootstrap/sxcw.c `
    spectrelib/csources/panic_handler.c `
    spectrelib/csources/yyjson_shim.c `
    (Join-Path $YyjsonSrc "yyjson.c") `
    -I $YyjsonSrc `
    -lDbghelp `
    -lbcrypt `
    -o stage0.exe
if (-not (Test-Path "stage0.exe")) {
    throw "bootstrap compiler build failed"
}

Write-Host "Building spectre stage 1..."
./stage0.exe ./src/sxc.sx -alt -runtime:hosted -o ./spectre-stage1.exe
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "spectre-stage1.exe")) {
    throw "stage 1 build failed (exit $LASTEXITCODE)"
}

Write-Host "Building spectre-dev..."
./spectre-stage1.exe ./src/sxc.sx -alt -runtime:hosted -o ./spectre-dev.exe -d:GIT_HASH=$Hash
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "spectre-dev.exe")) {
    throw "stage 2 build failed (exit $LASTEXITCODE)"
}

Write-Host "Windows compiler build complete."

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
    throw "clang is required to build the Windows bootstrap compiler"
}
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    throw "cmake is required to build yyjson"
}

$TempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
if (-not $TempRoot) {
    $TempRoot = Join-Path $Root ".tmp"
}
$YyjsonDir = if ($env:YYJSON_DIR) { $env:YYJSON_DIR } else { Join-Path $TempRoot "yyjson" }
$YyjsonLib = Join-Path $YyjsonDir "build\Release\yyjson.lib"

if (-not (Test-Path $YyjsonLib)) {
    if (-not (Test-Path $YyjsonDir)) {
        git clone --depth 1 https://github.com/ibireme/yyjson.git $YyjsonDir
    }
    cmake -S $YyjsonDir -B (Join-Path $YyjsonDir "build") `
        -DCMAKE_BUILD_TYPE=Release `
        -DBUILD_SHARED_LIBS=OFF `
        -DYYJSON_BUILD_TESTS=OFF `
        -DYYJSON_BUILD_FUZZER=OFF `
        -DYYJSON_BUILD_MISC=OFF `
        -DYYJSON_BUILD_DOC=OFF
    cmake --build (Join-Path $YyjsonDir "build") --config Release
}

if (-not (Test-Path $YyjsonLib)) {
    $found = Get-ChildItem -Path $YyjsonDir -Filter yyjson.lib -Recurse | Select-Object -First 1
    if ($found) {
        $YyjsonLib = $found.FullName
    }
}
if (-not (Test-Path $YyjsonLib)) {
    throw "could not find yyjson.lib under $YyjsonDir"
}

$Hash = "unknown"
try {
    $Hash = (git rev-parse --short HEAD).Trim()
} catch {
}

Write-Host "Building Windows bootstrap compiler..."
clang -O3 -Wno-everything `
    bootstrap/sxcw.c `
    spectrelib/csources/panic_handler.c `
    spectrelib/csources/yyjson_shim.c `
    -I (Join-Path $YyjsonDir "src") `
    $YyjsonLib `
    -lDbghelp `
    -o stage0.exe
if (-not (Test-Path "stage0.exe")) {
    throw "bootstrap compiler build failed"
}

Write-Host "Building spectre stage 1..."
./stage0.exe ./src/sxc.sx -alt -o ./spectre-stage1.exe
if (-not (Test-Path "spectre-stage1.exe")) {
    throw "stage 1 build failed"
}

Write-Host "Building spectre-dev..."
./spectre-stage1.exe ./src/sxc.sx -alt -o ./spectre-dev.exe -d:GIT_HASH=$Hash
if (-not (Test-Path "spectre-dev.exe")) {
    throw "stage 2 build failed"
}

Write-Host "Windows compiler build complete."

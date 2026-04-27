$ErrorActionPreference = "Stop"

$PREFIX    = "$env:USERPROFILE\.local"
$BIN_DIR   = "$PREFIX\bin"
$SRC_DIR   = "$PREFIX\src"

$SPECTRE_REPO = "https://github.com/spectrelang/spectre.git"
$YYJSON_REPO  = "https://github.com/ibireme/yyjson.git"

function Log($msg) {
    Write-Host "[INFO] $msg" -ForegroundColor Cyan
}

function Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
    exit 1
}

function Require-Cmd($cmd) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        Err "Missing required command: $cmd"
    }
}

function Ensure-Tool($cmd, $wingetId) {
    if (Get-Command $cmd -ErrorAction SilentlyContinue) {
        Log "$cmd already installed."
        return
    }

    Log "$cmd not found. Installing via winget..."
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Err "winget is required to install $cmd automatically."
    }

    winget install --id $wingetId -e --source winget
}

Log "Checking required tools..."

Ensure-Tool git Git.Git
Ensure-Tool cmake Kitware.CMake

Ensure-Tool clang LLVM.LLVM
Ensure-Tool tcc TinyCC.TinyCC

Require-Cmd git
Require-Cmd cmake

Log "Preparing directories..."
New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $SRC_DIR | Out-Null

Log "Installing Spectre (self-hosted)..."

$SPECTRE_DIR = "$SRC_DIR\spectre"

if (Test-Path $SPECTRE_DIR) {
    Log "Spectre source already exists, updating..."
    git -C $SPECTRE_DIR pull --ff-only
} else {
    git clone $SPECTRE_REPO $SPECTRE_DIR
}

Log "Installing yyjson..."

$YYJSON_DIR = "$SRC_DIR\yyjson"

if (Test-Path $YYJSON_DIR) {
    Log "yyjson source already exists, updating..."
    git -C $YYJSON_DIR pull --ff-only
} else {
    git clone $YYJSON_REPO $YYJSON_DIR
}

Log "Building yyjson..."
Remove-Item -Recurse -Force "$YYJSON_DIR\build" -ErrorAction SilentlyContinue

cmake -S $YYJSON_DIR -B "$YYJSON_DIR\build" `
    -DCMAKE_BUILD_TYPE=Release `
    -DBUILD_SHARED_LIBS=OFF `
    -DYYJSON_BUILD_TESTS=OFF `
    -DYYJSON_BUILD_FUZZER=OFF `
    -DYYJSON_BUILD_MISC=OFF `
    -DYYJSON_BUILD_DOC=OFF

cmake --build "$YYJSON_DIR\build" --config Release

$YYJSON_LIB = "$YYJSON_DIR\build\Release\yyjson.lib"
$YYJSON_INC = "$YYJSON_DIR\src"

if (-not (Test-Path $YYJSON_LIB)) {
    Err "yyjson build failed: missing library"
}

Log "Bootstrapping Spectre with clang..."

$BOOTSTRAP_SRC = "$SPECTRE_DIR\bootstrap\sxcw.c"
$BOOTSTRAP_OUT = "$SPECTRE_DIR\bootstrap\spectre.exe"

if (-not (Test-Path $BOOTSTRAP_SRC)) {
    Err "Missing bootstrap source at $BOOTSTRAP_SRC"
}

$CSOURCES_DIR = "$SPECTRE_DIR\std\csources"
$PANIC_HANDLER_SRC = "$CSOURCES_DIR\panic_handler.c"
$YYJSON_SHIM_SRC   = "$CSOURCES_DIR\yyjson_shim.c"

Log "Compiling bootstrap..."

clang -O3 -Wno-everything `
    $BOOTSTRAP_SRC `
    $PANIC_HANDLER_SRC `
    $YYJSON_SHIM_SRC `
    -I"$YYJSON_INC" `
    $YYJSON_LIB `
    -lDbghelp `
    -o $BOOTSTRAP_OUT

if (-not (Test-Path $BOOTSTRAP_OUT)) {
    Err "Bootstrap compilation failed"
}

Log "Installing Spectre binary..."

Copy-Item $BOOTSTRAP_OUT "$BIN_DIR\spectre.exe" -Force

$STDLIB_SRC  = "$SPECTRE_DIR\std"
$STDLIB_DEST = "$BIN_DIR\std"

if (-not (Test-Path $STDLIB_SRC)) {
    Err "Spectre std library not found"
}

Log "Installing standard library..."

Remove-Item -Recurse -Force $STDLIB_DEST -ErrorAction SilentlyContinue
Copy-Item $STDLIB_SRC $STDLIB_DEST -Recurse

if ($env:PATH -notlike "*$BIN_DIR*") {
    Log "IMPORTANT: $BIN_DIR is not in your PATH."
    Write-Host ""
    Write-Host "Add it with:"
    Write-Host "  setx PATH `"$BIN_DIR;%PATH%`""
    Write-Host ""
}

Log "Installation complete."

Write-Host ""
Write-Host "Installed:"
Write-Host "  - spectre -> $BIN_DIR\spectre.exe"
Write-Host "  - stdlib  -> $BIN_DIR\std"
Write-Host "  - clang   -> (system)"
Write-Host "  - tcc     -> (system)"
Write-Host "  - yyjson  -> $YYJSON_LIB"
Write-Host ""
Write-Host "Verify with:"
Write-Host "  spectre -v"

$ErrorActionPreference = "Stop"

$PREFIX       = "$env:USERPROFILE\.local"
$BIN_DIR      = "$PREFIX\bin"
$SRC_DIR      = "$PREFIX\src"
$SPECTRE_REPO = "https://github.com/spectrelang/spectre.git"

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

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Log "$cmd not found. Installing via winget..."
        winget install --id $wingetId -e --source winget `
            --accept-package-agreements --accept-source-agreements
        return
    }

    if (Get-Command scoop -ErrorAction SilentlyContinue) {
        Log "$cmd not found. Installing via Scoop..."
        scoop bucket add main 2>$null
        scoop install $cmd
        return
    }

    Err "Missing required command: $cmd (install winget or Scoop, or install it manually)"
}

function Get-VsWherePath {
    $candidates = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe",
        "$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    $vswhere = Get-Command vswhere.exe -ErrorAction SilentlyContinue
    if ($vswhere) {
        return $vswhere.Source
    }

    return $null
}

function Find-Msvc {
    $vswhere = Get-VsWherePath
    if (-not $vswhere) {
        return $null
    }

    $path = & $vswhere -latest -products '*' `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath 2>$null

    if ($LASTEXITCODE -ne 0 -or -not $path) {
        return $null
    }

    $path = ($path | Select-Object -First 1).Trim()
    if (Test-Path (Join-Path $path "VC\Auxiliary\Build\vcvars64.bat")) {
        return $path
    }

    return $null
}

function Install-Msvc {
    $workloadArgs = @(
        "--quiet",
        "--wait",
        "--norestart",
        "--nocache",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--includeRecommended"
    )

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Log "Installing MSVC Build Tools via winget (this may take a while)..."
        winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
            --source winget `
            --accept-package-agreements --accept-source-agreements `
            --override ($workloadArgs -join " ")

        if (Find-Msvc) {
            return
        }

        Log "winget could not install the MSVC workload; trying the Visual Studio bootstrapper..."
    }

    $installer = Join-Path $env:TEMP "vs_buildtools.exe"
    $url = "https://aka.ms/vs/17/release/vs_buildtools.exe"

    Log "Downloading the Visual Studio Build Tools bootstrapper..."
    Invoke-WebRequest -Uri $url -OutFile $installer -UseBasicParsing
    Unblock-File -Path $installer

    Log "Running the Visual Studio Build Tools installer (this may take a while)..."
    $proc = Start-Process -FilePath $installer -ArgumentList $workloadArgs -Wait -PassThru
    Remove-Item -Force $installer -ErrorAction SilentlyContinue

    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
        Err "MSVC Build Tools installation failed (exit $($proc.ExitCode))."
    }
}

function Import-MsvcEnvironment($vsPath) {
    $vcvars = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
    if (-not (Test-Path $vcvars)) {
        Err "Could not find vcvars64.bat under $vsPath"
    }

    $envDump = cmd.exe /d /c "`"$vcvars`" >nul 2>&1 && set"
    if ($LASTEXITCODE -ne 0) {
        Err "Failed to initialize the MSVC environment."
    }

    foreach ($line in $envDump) {
        $separator = $line.IndexOf("=")
        if ($separator -lt 1) {
            continue
        }

        $name = $line.Substring(0, $separator)
        $value = $line.Substring($separator + 1)
        Set-Item -Path "Env:$name" -Value $value
    }

    Log "MSVC environment initialized from $vsPath."
}

function Ensure-Msvc {
    $vsPath = Find-Msvc

    if ($vsPath) {
        Log "MSVC toolchain already installed."
    } else {
        Log "MSVC toolchain not found."
        Install-Msvc

        $vsPath = Find-Msvc
        if (-not $vsPath) {
            Err "MSVC Build Tools were installed but could not be located."
        }

        Log "MSVC toolchain installed."
    }

    Import-MsvcEnvironment $vsPath
    $script:MSVC_PATH = $vsPath

    if (-not (Get-Command cl -ErrorAction SilentlyContinue)) {
        Log "MSVC was found, but cl.exe is not available after initialization."
    }
}

function Add-LlvmToPath {
    $llvmCandidates = @(
        "$PREFIX\llvm\bin",
        "$env:ProgramFiles\LLVM\bin",
        "$env:LOCALAPPDATA\Programs\LLVM\bin"
    )

    $paths = @()
    foreach ($llvmBin in $llvmCandidates) {
        if ((Test-Path (Join-Path $llvmBin "clang.exe")) -and
            ($env:PATH -notlike "*$llvmBin*")) {
            $paths += $llvmBin
        }
    }

    if ($paths.Count -gt 0) {
        $env:PATH = ($paths -join ";") + ";" + $env:PATH
    }
}

function Get-LatestLlvmArchive {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    try {
        $releases = Invoke-RestMethod `
            -Uri "https://api.github.com/repos/llvm/llvm-project/releases?per_page=20" `
            -Headers @{ "User-Agent" = "spectre-install" }
    } catch {
        Log "Could not query the LLVM release list: $($_.Exception.Message)"
        return $null
    }

    foreach ($release in $releases) {
        if ($release.draft -or $release.prerelease) {
            continue
        }

        $asset = $release.assets |
            Where-Object { $_.name -match '^clang\+llvm-.*-x86_64-pc-windows-msvc\.tar\.xz$' } |
            Select-Object -First 1

        if ($asset) {
            return $asset
        }
    }

    return $null
}

function Expand-LlvmArchive($archive, $destination) {
    Remove-Item -Recurse -Force $destination -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $destination | Out-Null

    tar -xf $archive -C $destination --strip-components=1
    if ($LASTEXITCODE -eq 0 -and (Test-Path (Join-Path $destination "bin\clang.exe"))) {
        return
    }

    Log "Windows tar cannot extract .xz archives; using 7zr instead..."
    Remove-Item -Recurse -Force $destination -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $destination | Out-Null

    $sevenZip = Join-Path $env:TEMP "7zr.exe"
    if (-not (Test-Path $sevenZip)) {
        Invoke-WebRequest -Uri "https://www.7-zip.org/a/7zr.exe" `
            -OutFile $sevenZip -UseBasicParsing
        Unblock-File -Path $sevenZip
    }

    $tarDir = Join-Path $env:TEMP "llvm-tar"
    Remove-Item -Recurse -Force $tarDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $tarDir | Out-Null

    & $sevenZip x $archive "-o$tarDir" -y | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Remove-Item -Recurse -Force $tarDir -ErrorAction SilentlyContinue
        Err "7zr failed to decompress the LLVM archive."
    }

    $tarFile = Get-ChildItem -Path $tarDir -Filter "*.tar" | Select-Object -First 1
    if (-not $tarFile) {
        Remove-Item -Recurse -Force $tarDir -ErrorAction SilentlyContinue
        Err "Could not find the decompressed LLVM tar archive."
    }

    tar -xf $tarFile.FullName -C $destination --strip-components=1
    Remove-Item -Recurse -Force $tarDir -ErrorAction SilentlyContinue

    if ($LASTEXITCODE -ne 0 -or -not (Test-Path (Join-Path $destination "bin\clang.exe"))) {
        Err "Failed to extract the LLVM archive."
    }

    Remove-Item -Force $sevenZip -ErrorAction SilentlyContinue
}

function Install-LlvmPortable {
    $asset = Get-LatestLlvmArchive
    if (-not $asset) {
        Err "Could not find a portable LLVM Windows x64 archive."
    }

    $archive = Join-Path $env:TEMP $asset.name
    $llvmDir = "$PREFIX\llvm"
    $stagingDir = "$PREFIX\llvm.tmp"

    Log "Downloading $($asset.name)..."
    $ProgressPreference = "SilentlyContinue"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archive -UseBasicParsing

    if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
        Err "tar.exe is required to extract the LLVM archive."
    }

    Log "Extracting LLVM..."
    Expand-LlvmArchive $archive $stagingDir

    Remove-Item -Recurse -Force $llvmDir -ErrorAction SilentlyContinue
    Move-Item $stagingDir $llvmDir
    Remove-Item -Force $archive -ErrorAction SilentlyContinue

    Log "LLVM installed to $llvmDir."
}

function Test-Clang {
    $clang = Get-Command clang -ErrorAction SilentlyContinue
    if (-not $clang) {
        return $false
    }

    try {
        & $clang.Source --version *> $null
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    }
}

function Ensure-Clang {
    Add-LlvmToPath
    if (Test-Clang) {
        Log "clang already installed."
        return
    }

    Log "clang not found. Installing portable LLVM..."
    Install-LlvmPortable
    Add-LlvmToPath

    if (Test-Clang) {
        Log "clang installed."
        return
    }

    Err "Failed to install clang."
}

function Ensure-Tcc {
    if (Get-Command tcc -ErrorAction SilentlyContinue) {
        Log "tcc already installed."
        return
    }

    Log "tcc not found. Trying winget..."

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        try {
            winget install --id TinyCC.TinyCC -e --source winget `
                --accept-package-agreements --accept-source-agreements
        } catch {
            Log "winget install failed or package unavailable."
        }
    }

    if (Get-Command tcc -ErrorAction SilentlyContinue) {
        Log "tcc installed via winget."
        return
    }

    if (Get-Command scoop -ErrorAction SilentlyContinue) {
        Log "Trying Scoop..."
        scoop bucket add main 2>$null
        scoop install tcc
    } else {
        Log "Scoop not found; skipping optional tcc installation."
    }

    if (Get-Command tcc -ErrorAction SilentlyContinue) {
        Log "tcc installed via Scoop."
        return
    }

    Log "tcc unavailable; clang or MSVC will be used as the alternate backend."
}

Log "Checking required tools..."

Ensure-Tool git Git.Git
Ensure-Msvc
Ensure-Clang
Ensure-Tcc

Require-Cmd git
Require-Cmd clang

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

Log "Bootstrapping Spectre..."

$BOOTSTRAP_SRC = "$SPECTRE_DIR\bootstrap\sxcw.c"
$BOOTSTRAP_OUT = "$SPECTRE_DIR\bootstrap\spectre.exe"

if (-not (Test-Path $BOOTSTRAP_SRC)) {
    Err "Missing bootstrap source at $BOOTSTRAP_SRC"
}

$CSOURCES_DIR = "$SPECTRE_DIR\spectrelib\csources"
$PANIC_HANDLER_SRC = "$CSOURCES_DIR\panic_handler.c"

Log "Compiling bootstrap..."

clang -O3 -Wno-everything `
    $BOOTSTRAP_SRC `
    $PANIC_HANDLER_SRC `
    -lDbghelp `
    -lbcrypt `
    -o $BOOTSTRAP_OUT

if ($LASTEXITCODE -ne 0 -or -not (Test-Path $BOOTSTRAP_OUT)) {
    Err "Bootstrap compilation failed. On Windows, clang requires the MSVC/Windows SDK headers and libraries."
}

$STAGE0 = "$SPECTRE_DIR\spectre-stage0.exe"
$STAGE1 = "$SPECTRE_DIR\spectre-stage1.exe"
$STAGE2 = "$SPECTRE_DIR\spectre-stage2.exe"

Log "Building Spectre stage 1 from source..."

Copy-Item $BOOTSTRAP_OUT $STAGE0 -Force

Push-Location $SPECTRE_DIR
try {
    & .\spectre-stage0.exe .\src\sxc.sx -alt -runtime:hosted -o .\spectre-stage1.exe
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path .\spectre-stage1.exe)) {
        Err "Stage 1 build failed"
    }

    $GitHash = "unknown"
    try {
        $GitHash = (git rev-parse --short HEAD).Trim()
    } catch {
    }

    Log "Building Spectre stage 2 from source..."
    & .\spectre-stage1.exe "-d:GIT_HASH=$GitHash" .\src\sxc.sx -alt -runtime:hosted -o .\spectre-stage2.exe
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path .\spectre-stage2.exe)) {
        Err "Stage 2 build failed"
    }
} finally {
    Pop-Location
}

Log "Installing Spectre binary..."

Copy-Item $STAGE2 "$BIN_DIR\spectre.exe" -Force
Remove-Item -Force $STAGE0, $STAGE1, $STAGE2 -ErrorAction SilentlyContinue

$STDLIB_SRC  = "$SPECTRE_DIR\spectrelib"
$STDLIB_DEST = "$BIN_DIR\spectrelib"

if (-not (Test-Path $STDLIB_SRC)) {
    Err "Spectre std library not found"
}

Log "Installing standard library..."

Remove-Item -Recurse -Force $STDLIB_DEST -ErrorAction SilentlyContinue
Copy-Item $STDLIB_SRC $STDLIB_DEST -Recurse

$LlvmBin = "$PREFIX\llvm\bin"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $UserPath) {
    $UserPath = ""
}

$MissingPaths = @()
if ($UserPath -notlike "*$BIN_DIR*") {
    $MissingPaths += $BIN_DIR
}
if ((Test-Path (Join-Path $LlvmBin "clang.exe")) -and ($UserPath -notlike "*$LlvmBin*")) {
    $MissingPaths += $LlvmBin
}

if ($MissingPaths.Count -gt 0) {
    $PathPrefix = $MissingPaths -join ";"
    Log "IMPORTANT: Some install directories are not in your user PATH."
    Write-Host ""
    Write-Host "Add them with:"
    Write-Host "  setx PATH `"$PathPrefix;%PATH%`""
    Write-Host ""
}

Log "Installation complete."

$ClangPath = "(not installed)"
$ClangCmd = Get-Command clang -ErrorAction SilentlyContinue
if ($ClangCmd) {
    $ClangPath = $ClangCmd.Source
}

$TccPath = "(not installed)"
$TccCmd = Get-Command tcc -ErrorAction SilentlyContinue
if ($TccCmd) {
    $TccPath = $TccCmd.Source
}

Write-Host ""
Write-Host "Installed:"
Write-Host "  - spectre -> $BIN_DIR\spectre.exe"
Write-Host "  - stdlib  -> $BIN_DIR\spectrelib"
Write-Host "  - clang   -> $ClangPath"
Write-Host "  - msvc    -> $script:MSVC_PATH"
Write-Host "  - tcc     -> $TccPath"
Write-Host ""
Write-Host "Verify with:"
Write-Host "  spectre -v"

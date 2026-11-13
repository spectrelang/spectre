$ErrorActionPreference = "Stop"

$USER_PROFILE = [System.Environment]::GetFolderPath(
    [System.Environment+SpecialFolder]::UserProfile
)
if (-not $USER_PROFILE) {
    $USER_PROFILE = $env:USERPROFILE
}

$PREFIX       = Join-Path $USER_PROFILE ".local"
$BIN_DIR      = Join-Path $PREFIX "bin"
$SRC_DIR      = Join-Path $PREFIX "src"
$TEMP_DIR     = Join-Path $PREFIX "tmp"
$SPECTRE_REPO = "https://github.com/spectrelang/spectre.git"
$QBE_REPO     = "https://github.com/spectrelang/qbe-mirror.git"

function Log($msg) {
    Write-Host "[INFO] $msg" -ForegroundColor Cyan
}

function Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
    exit 1
}

function Ensure-Directory($path) {
    [System.IO.Directory]::CreateDirectory($path) | Out-Null
}

function Remove-Directory($path) {
    if ($path -and [System.IO.Directory]::Exists($path)) {
        [System.IO.Directory]::Delete($path, $true)
    }
}

function Remove-File($path) {
    if ($path -and [System.IO.File]::Exists($path)) {
        [System.IO.File]::Delete($path)
    }
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

    Ensure-Directory $TEMP_DIR
    $installer = Join-Path $TEMP_DIR "vs_buildtools.exe"
    $url = "https://aka.ms/vs/17/release/vs_buildtools.exe"

    Log "Downloading the Visual Studio Build Tools bootstrapper..."
    Invoke-WebRequest -Uri $url -OutFile $installer -UseBasicParsing
    Unblock-File -Path $installer

    Log "Running the Visual Studio Build Tools installer (this may take a while)..."
    $proc = Start-Process -FilePath $installer -ArgumentList $workloadArgs -Wait -PassThru
    Remove-File $installer

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
        if ([System.IO.File]::Exists((Join-Path $llvmBin "clang.exe")) -and
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
    Remove-Directory $destination
    Ensure-Directory $destination

    tar -xf $archive -C $destination --strip-components=1
    if ($LASTEXITCODE -eq 0 -and [System.IO.File]::Exists(
        (Join-Path $destination "bin\clang.exe"))) {
        return
    }

    Log "Windows tar cannot extract .xz archives; using 7zr instead..."
    Remove-Directory $destination
    Ensure-Directory $destination
    Ensure-Directory $TEMP_DIR

    $sevenZip = Join-Path $TEMP_DIR "7zr.exe"
    if (-not [System.IO.File]::Exists($sevenZip)) {
        Invoke-WebRequest -Uri "https://www.7-zip.org/a/7zr.exe" `
            -OutFile $sevenZip -UseBasicParsing
        Unblock-File -Path $sevenZip
    }

    $tarDir = Join-Path $TEMP_DIR "llvm-tar"
    Remove-Directory $tarDir
    Ensure-Directory $tarDir

    & $sevenZip x $archive "-o$tarDir" -y | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Remove-Directory $tarDir
        Err "7zr failed to decompress the LLVM archive."
    }

    $tarFiles = [System.IO.Directory]::GetFiles($tarDir, "*.tar")
    if ($tarFiles.Count -eq 0) {
        Remove-Directory $tarDir
        Err "Could not find the decompressed LLVM tar archive."
    }

    tar -xf $tarFiles[0] -C $destination --strip-components=1
    Remove-Directory $tarDir

    if ($LASTEXITCODE -ne 0 -or -not [System.IO.File]::Exists(
        (Join-Path $destination "bin\clang.exe"))) {
        Err "Failed to extract the LLVM archive."
    }

    Remove-File $sevenZip
}

function Install-LlvmPortable {
    $asset = Get-LatestLlvmArchive
    if (-not $asset) {
        Err "Could not find a portable LLVM Windows x64 archive."
    }

    Ensure-Directory $TEMP_DIR
    $archive = Join-Path $TEMP_DIR $asset.name
    $llvmDir = Join-Path $PREFIX "llvm"
    $stagingDir = Join-Path $PREFIX "llvm.tmp"

    Log "Downloading $($asset.name)..."
    $ProgressPreference = "SilentlyContinue"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archive -UseBasicParsing

    if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
        Err "tar.exe is required to extract the LLVM archive."
    }

    Log "Extracting LLVM..."
    Expand-LlvmArchive $archive $stagingDir

    Remove-Directory $llvmDir
    [System.IO.Directory]::Move($stagingDir, $llvmDir)
    Remove-File $archive

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

function Build-Uqbe {
    $qbeDir = Join-Path $SRC_DIR "qbe"

    if (Test-Path $qbeDir) {
        Log "UQBE source already exists, updating..."
        git -C $qbeDir pull --ff-only
    } else {
        git clone $QBE_REPO $qbeDir
    }

    $qbeHash = "unknown"
    try {
        $qbeHash = (git -C $qbeDir rev-parse --short HEAD).Trim()
    } catch {
    }

    $qbeTarget = "T_amd64_win"
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        $qbeTarget = "T_arm64_win"
    }

    $config = "#define Deftgt $qbeTarget`n#define GIT_COMMIT `"$qbeHash`"`n"
    [System.IO.File]::WriteAllText(
        (Join-Path $qbeDir "config.h"),
        $config,
        [System.Text.Encoding]::ASCII
    )

    $sources = @()
    $sources += [System.IO.Directory]::GetFiles($qbeDir, "*.c")
    foreach ($arch in @("amd64", "arm64", "rv64")) {
        $archDir = Join-Path $qbeDir $arch
        if ([System.IO.Directory]::Exists($archDir)) {
            $sources += [System.IO.Directory]::GetFiles($archDir, "*.c")
        }
    }

    $uqbeOut = Join-Path $BIN_DIR "uqbe.exe"
    Log "Building UQBE from source..."

    Push-Location $qbeDir
    try {
        & clang -std=c99 -O2 -w @sources -o $uqbeOut
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path $uqbeOut)) {
            Err "UQBE build failed"
        }
    } finally {
        Pop-Location
    }

    Log "UQBE installed: $uqbeOut"
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
Ensure-Directory $BIN_DIR
Ensure-Directory $SRC_DIR

Build-Uqbe

if ($env:PATH -notlike "*$BIN_DIR*") {
    $env:PATH = "$BIN_DIR;$env:PATH"
}

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

Log "Compiling bootstrap..."

clang -O3 -Wno-everything `
    $BOOTSTRAP_SRC `
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
    & .\spectre-stage0.exe .\src\sxc.sx -runtime:hosted -o .\spectre-stage1.exe
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path .\spectre-stage1.exe)) {
        Err "Stage 1 build failed"
    }

    $GitHash = "unknown"
    try {
        $GitHash = (git rev-parse --short HEAD).Trim()
    } catch {
    }

    Log "Building Spectre stage 2 from source..."
    & .\spectre-stage1.exe "-d:GIT_HASH=$GitHash" .\src\sxc.sx -runtime:hosted -o .\spectre-stage2.exe
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path .\spectre-stage2.exe)) {
        Err "Stage 2 build failed"
    }
} finally {
    Pop-Location
}

Log "Installing Spectre binary..."

Copy-Item $STAGE2 "$BIN_DIR\spectre.exe" -Force
Remove-File $STAGE0
Remove-File $STAGE1
Remove-File $STAGE2

$STDLIB_SRC  = "$SPECTRE_DIR\spectrelib"
$STDLIB_DEST = "$BIN_DIR\spectrelib"

if (-not (Test-Path $STDLIB_SRC)) {
    Err "Spectre std library not found"
}

Log "Installing standard library..."

Remove-Directory $STDLIB_DEST
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
if ([System.IO.File]::Exists((Join-Path $LlvmBin "clang.exe")) -and
    ($UserPath -notlike "*$LlvmBin*")) {
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
Write-Host "  - uqbe    -> $BIN_DIR\uqbe.exe (default backend)"
Write-Host "  - clang   -> $ClangPath"
Write-Host "  - msvc    -> $script:MSVC_PATH"
Write-Host "  - tcc     -> $TccPath"
Write-Host ""
Write-Host "UQBE is the default backend on Windows."
Write-Host "Pass -alt to use a C backend instead."
Write-Host ""
Write-Host "Verify with:"
Write-Host "  spectre -v"

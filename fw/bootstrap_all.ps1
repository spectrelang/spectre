$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$Spectre = Join-Path $Root "spectre-dev.exe"
if (-not (Test-Path $Spectre)) {
    $Spectre = Join-Path $Root "spectre-dev"
}
if (-not (Test-Path $Spectre)) {
    throw "spectre-dev is missing"
}

$GitHash = "unknown"
try {
    $GitHash = (git rev-parse --short HEAD).Trim()
} catch {
}

$ProjectVersion = ""
$SxMod = Get-Content -Path "sx.mod" -ErrorAction SilentlyContinue
foreach ($line in $SxMod) {
    if ($line -match '^\s*version\s+"([^"]+)"') {
        $ProjectVersion = $Matches[1]
        break
    }
}

$Defines = @("-d:GIT_HASH=$GitHash")
if ($ProjectVersion) {
    $Defines += "-d:SPECTRE_MOD_VERSION=$ProjectVersion"
}

New-Item -ItemType Directory -Force -Path "bootstrap" | Out-Null

function Emit-File {
    param(
        [string]$Target,
        [string[]]$Arguments
    )

    $Tmp = "$Target.tmp.$PID"
    Remove-Item -Force $Tmp -ErrorAction SilentlyContinue
    & $Spectre @Arguments -o $Tmp
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Tmp)) {
        Remove-Item -Force $Tmp -ErrorAction SilentlyContinue
        throw "failed to generate $Target"
    }
    Move-Item -Force $Tmp $Target
}

$Common = @($Defines) + @("./src/sxc.sx", "-b")

Emit-File "bootstrap/sxc.ssa" ($Common + @(
    "-emit-ssa",
    "-emit-shim:spectrelib/csources/qbe_runtime.c",
    "-src-target:linux"
))
Emit-File "bootstrap/sxc.c" ($Common + @(
    "-emit-alt",
    "-src-target:linux"
))
Emit-File "bootstrap/sxc_darwin.ssa" ($Common + @(
    "-emit-ssa",
    "-src-target:darwin"
))
Emit-File "bootstrap/sxc_darwin.c" ($Common + @(
    "-emit-alt",
    "-src-target:darwin"
))
Emit-File "bootstrap/sxcw.c" ($Common + @(
    "-emit-alt",
    "-src-target:windows"
))

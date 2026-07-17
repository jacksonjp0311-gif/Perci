[CmdletBinding()]
param(
    [ValidateSet('chat', 'status', 'test', 'bench', 'intel', 'cortex-init')]
    [string]$Mode = 'chat'
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $Root

try {
    $env:PYTHONUTF8 = '1'
    $env:PERCI_CORTEX_MODE = if ($env:PERCI_CORTEX_MODE) { $env:PERCI_CORTEX_MODE } else { 'auto' }

    $V3Weights = Join-Path $Root 'models\perci-cognitive-v0.3.pwgt'
    $V2Weights = Join-Path $Root 'models\perci-cognitive-v0.2.pwgt'
    $V1Weights = Join-Path $Root 'models\perci-cognitive-v0.1.pwgt'
    $Weights = if ($env:PERCI_WEIGHTS) {
        $env:PERCI_WEIGHTS
    } elseif (Test-Path -LiteralPath $V3Weights -PathType Leaf) {
        $V3Weights
    } elseif (Test-Path -LiteralPath $V2Weights -PathType Leaf) {
        $V2Weights
    } else {
        $V1Weights
    }
    if (-not (Test-Path -LiteralPath $Weights -PathType Leaf)) {
        throw "Perci weights are missing: $Weights"
    }

    $Stream = [System.IO.File]::OpenRead($Weights)
    try {
        $MagicBytes = New-Object byte[] 8
        if ($Stream.Read($MagicBytes, 0, 8) -ne 8) {
            throw "Perci weights are truncated: $Weights"
        }
        $Magic = [Text.Encoding]::ASCII.GetString($MagicBytes)
    } finally {
        $Stream.Dispose()
    }
    if ($Magic -notin @('PERCIW01', 'PERCIW02', 'PERCIW03')) {
        throw "Perci weights have an unknown signature '$Magic': $Weights"
    }
    $env:PERCI_WEIGHTS = $Weights

    $CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    $CargoExe = Join-Path $CargoBin 'cargo.exe'
    if ((Test-Path -LiteralPath $CargoExe) -and (($env:Path -split ';') -notcontains $CargoBin)) {
        $env:Path = "$CargoBin;$env:Path"
    }

    $CortexRoot = Join-Path $Root 'Cortex'
    $CortexHome = Join-Path $Root '.perci\cortex-home'
    $CortexPython = Join-Path $CortexRoot '.venv\Scripts\python.exe'
    $env:PERCI_CORTEX_ROOT = $CortexRoot
    $env:PERCI_CORTEX_HOME = $CortexHome
    $env:PERCI_CORTEX_PYTHON = $CortexPython
    $env:CORTEX_HOME = $CortexHome
    $env:PERCI_CORTEX_REPO = 'Perci'

    if ($Mode -eq 'cortex-init') {
        & (Join-Path $Root 'Initialize-Perci-Cortex.ps1')
        exit $LASTEXITCODE
    }

    $Cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $Cargo) {
        throw 'Rust/Cargo is required for first build or source updates.'
    }

    if ($Mode -eq 'test') {
        & cargo test --release
        exit $LASTEXITCODE
    }

    # Always let Cargo verify source freshness. Timestamp-only checks can select
    # a stale executable after source files are copied, restored, or synced.
    $LiveTarget = if ($env:PERCI_TARGET_DIR) {
        $env:PERCI_TARGET_DIR
    } else {
        Join-Path $Root 'target\live'
    }
    $env:CARGO_TARGET_DIR = $LiveTarget
    $Exe = Join-Path $LiveTarget 'release\perci.exe'
    # Build quietly when possible; brief status only (will be cleared before chat).
    Write-Host 'Synchronizing optimized Perci runtime...' -ForegroundColor DarkRed
    & cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw 'Perci release build failed. Exit any other live Perci window and retry.'
    }

    if ($Mode -eq 'status') {
        Clear-Host
        & $Exe status
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'bench') {
        & $Exe bench
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'intel') {
        Clear-Host
        & $Exe intel
        exit $LASTEXITCODE
    }

    # Fade out preamble: wipe PS copyright + cargo lines so Perci snaps to top.
    $ver = 'Perci'
    try {
        $toml = Get-Content (Join-Path $Root 'Cargo.toml') -Raw
        if ($toml -match 'version\s*=\s*"([^"]+)"') { $ver = "Perci v$($Matches[1])" }
    } catch {}
    $Host.UI.RawUI.WindowTitle = "$ver // dark-blood"
    $env:PERCI_COLOR = if ($env:PERCI_COLOR) { $env:PERCI_COLOR } else { 'always' }
    Clear-Host
    # Soft beat so clear is perceived as a transition, then chat paints the banner.
    Start-Sleep -Milliseconds 80
    & $Exe chat
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}

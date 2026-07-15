[CmdletBinding()]
param(
    [ValidateSet('chat', 'status', 'test', 'cortex-init')]
    [string]$Mode = 'chat'
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $Root

try {
    $Weights = Join-Path $Root 'models\perci-cognitive-v0.1.pwgt'
    if (-not (Test-Path -LiteralPath $Weights -PathType Leaf)) {
        throw "Perci weights are missing: $Weights"
    }

    $ExpectedBytes = 209715200
    $ActualBytes = (Get-Item -LiteralPath $Weights).Length
    if ($ActualBytes -ne $ExpectedBytes) {
        throw "Perci weights have the wrong size. Expected $ExpectedBytes bytes; found $ActualBytes."
    }

    # Rustup commonly exists but its directory is missing from inherited PATH.
    $CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    $CargoExe = Join-Path $CargoBin 'cargo.exe'
    if ((Test-Path -LiteralPath $CargoExe) -and (($env:Path -split ';') -notcontains $CargoBin)) {
        $env:Path = "$CargoBin;$env:Path"
    }

    $Cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $Cargo) {
        Write-Host 'Rust/Cargo is required to build the Rust interface.' -ForegroundColor Yellow
        Write-Host 'Install Rust from rustup or add ~/.cargo/bin to PATH.'
        exit 1
    }

    $CortexRoot = Join-Path $Root 'Cortex'
    $CortexHome = Join-Path $Root '.perci\cortex-home'
    $env:PERCI_CORTEX_ROOT = $CortexRoot
    $env:PERCI_CORTEX_HOME = $CortexHome
    $env:CORTEX_HOME = $CortexHome
    $env:PERCI_CORTEX_REPO = 'Perci'

    if ($Mode -eq 'cortex-init') {
        & (Join-Path $Root 'Initialize-Perci-Cortex.ps1')
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'test') {
        & cargo test --release
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'status') {
        & cargo run --release -- status
        exit $LASTEXITCODE
    }

    if (
        (Test-Path -LiteralPath (Join-Path $CortexRoot 'cortex\cli.py')) -and
        -not (Test-Path -LiteralPath (Join-Path $Root '.cortex\config.json'))
    ) {
        Write-Host 'Cortex is vendored but not initialized for this checkout.' -ForegroundColor Yellow
        Write-Host 'Run: .\Launch-Perci.ps1 -Mode cortex-init'
        Write-Host ''
    }

    & cargo run --release -- chat
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
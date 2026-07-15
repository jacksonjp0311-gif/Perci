[CmdletBinding()]
param(
    [ValidateSet('chat','status','test')]
    [string]$Mode = 'chat'
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $Root
try {
    $Weights = Join-Path $Root 'models\perci-cognitive-v0.1.pwgt'
    if (-not (Test-Path -LiteralPath $Weights)) {
        throw "Perci weights are missing: $Weights"
    }

    $ExpectedBytes = 209715200
    $ActualBytes = (Get-Item -LiteralPath $Weights).Length
    if ($ActualBytes -ne $ExpectedBytes) {
        throw "Perci weights have the wrong size. Expected $ExpectedBytes bytes; found $ActualBytes."
    }

    $Cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $Cargo) {
        Write-Host 'Rust/Cargo is required to run the Rust interface.' -ForegroundColor Yellow
        Write-Host 'Install Rust from rustup, reopen PowerShell, then run this script again.'
        Write-Host 'Windows option: winget install Rustlang.Rustup'
        exit 1
    }

    if ($Mode -eq 'test') {
        & cargo test --release
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'status') {
        & cargo run --release -- status
        exit $LASTEXITCODE
    }

    & cargo run --release -- chat
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}

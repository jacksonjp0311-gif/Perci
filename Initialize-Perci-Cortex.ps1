[CmdletBinding()]
param(
    [switch]$RunTests,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$Root = $PSScriptRoot
$CortexRoot = Join-Path $Root "Cortex"
$Launcher = Join-Path $CortexRoot "Cortex-All-One.ps1"

if (-not (Test-Path -LiteralPath $Launcher -PathType Leaf)) {
    throw "Vendored Cortex engine was not found: $Launcher"
}

$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    throw "Python 3.10 or newer is required for Cortex."
}

$VersionText = & $Python.Source --version 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "Python could not be executed."
}

$VersionMatch = [regex]::Match([string]$VersionText, '(\d+)\.(\d+)')
if (-not $VersionMatch.Success) {
    throw "Could not determine Python version: $VersionText"
}

$Major = [int]$VersionMatch.Groups[1].Value
$Minor = [int]$VersionMatch.Groups[2].Value
if ($Major -lt 3 -or ($Major -eq 3 -and $Minor -lt 10)) {
    throw "Cortex requires Python 3.10 or newer; found $VersionText"
}

$env:CORTEX_HOME = Join-Path $Root ".perci\cortex-home"
$env:PERCI_CORTEX_HOME = $env:CORTEX_HOME
$env:PERCI_CORTEX_ROOT = $CortexRoot
$env:PERCI_CORTEX_REPO = "Perci"

Write-Host ""
Write-Host "Initializing Cortex as Perci's governed memory organ..." -ForegroundColor Cyan

& $Launcher `
    -RepositoryPath $Root `
    -Name "Perci" `
    -Task "Map Perci, verify its environment, and prepare bounded provenance-bearing context" `
    -Python $Python.Source `
    -Force:$Force `
    -RunTests:$RunTests

if ($LASTEXITCODE -ne 0) {
    throw "Cortex initialization failed."
}

Write-Host ""
Write-Host "Cortex is attached to Perci." -ForegroundColor Green
Write-Host "Local state: $env:CORTEX_HOME"
Write-Host "Repository integration: $(Join-Path $Root '.cortex')"
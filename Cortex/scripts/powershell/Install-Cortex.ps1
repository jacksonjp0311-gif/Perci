param(
    [string]$Python = "python",
    [switch]$WithSemanticModel
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $Root

if (-not (Test-Path ".venv")) {
    & $Python -m venv .venv
}

$VenvPython = Join-Path $Root ".venv\Scripts\python.exe"
if ($WithSemanticModel) {
    & $VenvPython -m pip install ".[semantic]"
}
if (-not $WithSemanticModel) {
    & $VenvPython -m pip install .
}
& $VenvPython -m cortex init --json
& $VenvPython -m cortex doctor --json

Write-Host ""
Write-Host "Cortex installed successfully." -ForegroundColor Green
Write-Host "Activate the environment with:"
Write-Host "  $Root\.venv\Scripts\Activate.ps1"
Write-Host "Bootstrap a repository with:"
Write-Host "  .\scripts\powershell\Bootstrap-CortexRepo.ps1 -RepositoryPath 'C:\path\to\repo' -Name 'MyProject'"

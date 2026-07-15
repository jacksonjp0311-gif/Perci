param([string]$Python = "python")

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$VenvPython = Join-Path $Root ".venv\Scripts\python.exe"
if (Test-Path $VenvPython) { $Python = $VenvPython }
Set-Location $Root
& $Python -m compileall -q cortex tests
& $Python -m unittest discover -s tests -v
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Cortex tests passed." -ForegroundColor Green

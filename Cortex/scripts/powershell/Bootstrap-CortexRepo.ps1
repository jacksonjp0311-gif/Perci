param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath,
    [string]$Name = "",
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($env:PYTHONPATH)) { $env:PYTHONPATH = $Root }
if (-not [string]::IsNullOrWhiteSpace($env:PYTHONPATH) -and -not $env:PYTHONPATH.StartsWith($Root)) { $env:PYTHONPATH = "$Root;$env:PYTHONPATH" }
$VenvPython = Join-Path $Root ".venv\Scripts\python.exe"
$Python = "python"
if (Test-Path $VenvPython) { $Python = $VenvPython }

$ResolvedRepo = (Resolve-Path $RepositoryPath).Path
$ArgsList = @("-m", "cortex", "bootstrap", $ResolvedRepo, "--json")
if (-not [string]::IsNullOrWhiteSpace($Name)) { $ArgsList += @("--name", $Name) }
if ($Force) { $ArgsList += "--force" }

& $Python @ArgsList
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "Repository bootstrap complete." -ForegroundColor Green
Write-Host "From the target repository, activate Cortex with:"
Write-Host "  .\.cortex\bin\cortex.ps1 activate -Task '<current task>'"

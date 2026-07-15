param(
    [string]$RepositoryPath = "",
    [string]$Name = "",
    [string]$Task = "Map this repository, learn its environment, and prepare bounded agent context",
    [string]$Python = "python",
    [switch]$Force,
    [switch]$WithSemanticModel,
    [switch]$RunTests
)

$ErrorActionPreference = "Stop"
$Script = Join-Path $PSScriptRoot "scripts\powershell\Cortex-All-One.ps1"
& $Script `
    -RepositoryPath $RepositoryPath `
    -Name $Name `
    -Task $Task `
    -Python $Python `
    -Force:$Force `
    -WithSemanticModel:$WithSemanticModel `
    -RunTests:$RunTests
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

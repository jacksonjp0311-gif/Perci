param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath,
    [Parameter(Mandatory = $true)]
    [string]$Task,
    [int]$Budget = 1200
)

$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path $RepositoryPath).Path
$Wrapper = Join-Path $Repo ".cortex\bin\cortex.ps1"
if (-not (Test-Path $Wrapper)) {
    throw "Cortex is not integrated into $Repo. Run Bootstrap-CortexRepo.ps1 first."
}
& $Wrapper activate -Task $Task -Budget $Budget

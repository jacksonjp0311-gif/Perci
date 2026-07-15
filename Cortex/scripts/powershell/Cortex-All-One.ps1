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
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$OriginalLocation = (Get-Location).Path

function Test-ProjectRoot {
    param([string]$Path)

    $Markers = @(
        ".git",
        "pyproject.toml",
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "Makefile",
        "README.md"
    )

    foreach ($Marker in $Markers) {
        if (Test-Path -LiteralPath (Join-Path $Path $Marker)) {
            return $true
        }
    }

    return $false
}

function Resolve-RepositoryTarget {
    param([string]$RequestedPath)

    if (-not [string]::IsNullOrWhiteSpace($RequestedPath)) {
        return (Resolve-Path -LiteralPath $RequestedPath).Path
    }

    $Parent = Split-Path -Parent $Root
    if (Test-ProjectRoot -Path $Parent) {
        return (Resolve-Path -LiteralPath $Parent).Path
    }

    if ($OriginalLocation -ne $Root -and (Test-ProjectRoot -Path $OriginalLocation)) {
        return (Resolve-Path -LiteralPath $OriginalLocation).Path
    }

    return $Root
}

$Target = Resolve-RepositoryTarget -RequestedPath $RepositoryPath
if ([string]::IsNullOrWhiteSpace($Name)) {
    $Name = Split-Path -Leaf $Target
}

Set-Location $Root

if (-not (Test-Path -LiteralPath ".venv")) {
    & $Python -m venv .venv
    if ($LASTEXITCODE -ne 0) { throw "Failed to create the Cortex virtual environment." }
}

$VenvPython = Join-Path $Root ".venv\Scripts\python.exe"
if (-not (Test-Path -LiteralPath $VenvPython)) {
    throw "Cortex virtual-environment Python was not created: $VenvPython"
}

if ([string]::IsNullOrWhiteSpace($env:PYTHONPATH)) {
    $env:PYTHONPATH = $Root
}
if (-not [string]::IsNullOrWhiteSpace($env:PYTHONPATH) -and -not $env:PYTHONPATH.StartsWith($Root)) {
    $env:PYTHONPATH = "$Root;$env:PYTHONPATH"
}

if ($WithSemanticModel) {
    & $VenvPython -m pip install -e ".[semantic]"
    if ($LASTEXITCODE -ne 0) { throw "Failed to install the optional semantic model dependencies." }
}

& $VenvPython -c "import cortex; print(cortex.__version__)"
if ($LASTEXITCODE -ne 0) { throw "Cortex could not be imported from the portable engine folder." }

& $VenvPython -m cortex init --json
if ($LASTEXITCODE -ne 0) { throw "Cortex initialization failed." }

if ($RunTests) {
    & $VenvPython -m compileall -q cortex tests
    if ($LASTEXITCODE -ne 0) { throw "Compile validation failed." }
    & $VenvPython -m unittest discover -s tests -v
    if ($LASTEXITCODE -ne 0) { throw "Cortex tests failed." }
}

$BootstrapArgs = @(
    "-m", "cortex",
    "bootstrap", $Target,
    "--name", $Name,
    "--json"
)
if ($Force) { $BootstrapArgs += "--force" }

& $VenvPython @BootstrapArgs
if ($LASTEXITCODE -ne 0) { throw "Repository bootstrap failed." }

& $VenvPython -m cortex doctor --repo $Name --json
if ($LASTEXITCODE -ne 0) { throw "Cortex doctor failed." }

& $VenvPython -m cortex verify --repo $Name --json
if ($LASTEXITCODE -ne 0) { throw "Cortex verification failed." }

& $VenvPython -m cortex activate --repo $Name --task $Task --json
if ($LASTEXITCODE -ne 0) { throw "Initial Cortex activation failed." }

Set-Location $OriginalLocation

Write-Host ""
Write-Host "CORTEX + NEURAL INTERLINK READY" -ForegroundColor Green
Write-Host "Engine: $Root"
Write-Host "Integrated repository: $Target"
Write-Host "Repository name: $Name"
Write-Host ""
Write-Host "Use from the integrated repository:"
Write-Host "  .\.cortex\bin\cortex.ps1 activate -Task '<current task>'"

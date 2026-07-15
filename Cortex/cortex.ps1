param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path $PSScriptRoot).Path
$Python = Join-Path $Root ".venv\Scripts\python.exe"
if (-not (Test-Path -LiteralPath $Python)) {
    $PythonCommand = Get-Command python -ErrorAction SilentlyContinue
    if ($null -eq $PythonCommand) { throw "Python 3.10+ is required." }
    $Python = $PythonCommand.Source
}
if ([string]::IsNullOrWhiteSpace($env:PYTHONPATH)) {
    $env:PYTHONPATH = $Root
}
if (-not [string]::IsNullOrWhiteSpace($env:PYTHONPATH) -and -not $env:PYTHONPATH.StartsWith($Root)) {
    $env:PYTHONPATH = "$Root;$env:PYTHONPATH"
}
& $Python -m cortex @Arguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

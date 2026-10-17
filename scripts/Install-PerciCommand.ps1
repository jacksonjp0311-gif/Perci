<#
.SYNOPSIS
  Make `perci` work from any new PowerShell / cmd / Windows Terminal window.

.DESCRIPTION
  1. Builds the release binary once (cargo build --release -> target\release\perci.exe).
  2. Writes a small shim, %USERPROFILE%\.local\bin\perci.cmd, that runs that exe
     from the repo folder with the same environment Launch-Perci.ps1 uses.
     `perci` with no arguments opens chat; `perci ask ...`, `perci status` etc.
     pass straight through; `perci --rebuild` rebuilds the release exe.
  3. Adds %USERPROFILE%\.local\bin to the USER Path (idempotent, no admin).
  4. Best-effort: creates/repairs a Desktop "Perci" shortcut to Start-Perci.cmd.

  Path-agnostic: the repo root is the parent of this script's folder, so it
  works wherever the repo is cloned. Re-run it after moving the repo.

.PARAMETER Rebuild
  Only rebuild the release exe (used by `perci --rebuild`). Skips PATH,
  shim and shortcut steps.

.PARAMETER NoShortcut
  Skip the Desktop shortcut step.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Install-PerciCommand.ps1
#>
[CmdletBinding()]
param(
    [switch]$Rebuild,
    [switch]$NoShortcut
)

$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
$CargoToml = Join-Path $Root 'Cargo.toml'
if (-not (Test-Path -LiteralPath $CargoToml) -or -not ((Get-Content -LiteralPath $CargoToml -Raw) -match 'name\s*=\s*"perci"')) {
    Write-Host "  This script must live in <Perci repo>\scripts. Could not find Perci's Cargo.toml in $Root" -ForegroundColor Red
    exit 1
}

$TargetRoot = Join-Path $Root 'target'
$Exe = Join-Path $TargetRoot 'release\perci.exe'
$BinDir = Join-Path $env:USERPROFILE '.local\bin'
$Shim = Join-Path $BinDir 'perci.cmd'

function Write-Step([string]$Text) { Write-Host "  $Text" -ForegroundColor Cyan }
function Write-Ok([string]$Text) { Write-Host "  ok  $Text" -ForegroundColor Green }
function Write-Note([string]$Text) { Write-Host "  !   $Text" -ForegroundColor Yellow }

function Test-PerciWeights {
    # Fresh clones without Git LFS get small pointer files instead of real weights.
    $models = Join-Path $Root 'models'
    $candidates = @('perci-cognitive-v0.3.pwgt', 'perci-cognitive-v0.2.pwgt', 'perci-cognitive-v0.1.pwgt') |
        ForEach-Object { Join-Path $models $_ } |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
    if (-not $candidates) {
        Write-Note "No weights found in $models. Perci will not answer until they are present (git lfs pull)."
        return
    }
    $first = @($candidates)[0]
    $bytes = New-Object byte[] 8
    $fs = [System.IO.File]::OpenRead($first)
    try { $n = $fs.Read($bytes, 0, 8) } finally { $fs.Dispose() }
    $magic = [Text.Encoding]::ASCII.GetString($bytes, 0, $n)
    if ($magic -in @('PERCIW01', 'PERCIW02', 'PERCIW03')) {
        Write-Ok ("weights {0} ({1:N0} MB)" -f (Split-Path -Leaf $first), ((Get-Item -LiteralPath $first).Length / 1MB))
    } elseif ($magic -like 'version*') {
        Write-Note "Weights are Git LFS pointers, not real files. Fix with:  cd `"$Root`"; git lfs install; git lfs pull"
    } else {
        Write-Note "Weights file has an unknown signature '$magic': $first"
    }
}

function Stop-PerciForRebuild {
    # Windows locks a running exe, so a rebuild fails while Perci is open.
    $running = @(Get-Process -Name 'perci' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and ($_.Path -like "$Root*") })
    if ($running.Count -gt 0) {
        Write-Note ("Closing {0} running Perci process(es) so the exe can be replaced." -f $running.Count)
        $running | ForEach-Object { try { Stop-Process -Id $_.Id -Force -ErrorAction Stop } catch {} }
        Start-Sleep -Milliseconds 300
    }
}

function Invoke-PerciReleaseBuild {
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if ((Test-Path -LiteralPath (Join-Path $cargoBin 'cargo.exe')) -and (($env:Path -split ';') -notcontains $cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host '  Rust/Cargo is not installed. Install it from https://rustup.rs, open a new terminal, and re-run this script.' -ForegroundColor Red
        exit 1
    }
    Stop-PerciForRebuild
    Write-Step 'Building Perci (cargo build --release). The first build can take a few minutes...'
    $env:CARGO_TARGET_DIR = $TargetRoot
    Push-Location $Root
    try {
        & cargo build --release
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    if ($code -ne 0) {
        Write-Host ''
        Write-Host "  Build failed (cargo exit $code). Fix the error above, then re-run:" -ForegroundColor Red
        Write-Host "    powershell -NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -ForegroundColor Gray
        exit $code
    }
    if (-not (Test-Path -LiteralPath $Exe)) {
        Write-Host "  Build reported success but $Exe is missing." -ForegroundColor Red
        exit 1
    }
    $ver = ''
    try { $ver = (& $Exe --version 2>$null | Select-Object -First 1) } catch {}
    Write-Ok ("built {0}" -f $(if ($ver) { $ver } else { $Exe }))
}

function Write-PerciShim {
    $null = New-Item -ItemType Directory -Force -Path $BinDir
    # ASCII + CRLF so cmd.exe parses it reliably. Root is baked in at install time.
    $lines = @(
        '@echo off',
        'rem Generated by Perci scripts\Install-PerciCommand.ps1 - re-run that script to update.',
        'setlocal',
        "set `"PERCI_ROOT=$Root`"",
        'set "PERCI_EXE=%PERCI_ROOT%\target\release\perci.exe"',
        'if /i "%~1"=="--rebuild" goto rebuild',
        'if not exist "%PERCI_EXE%" goto notbuilt',
        'set "PYTHONUTF8=1"',
        'if not defined PERCI_CORTEX_MODE set "PERCI_CORTEX_MODE=auto"',
        'set "PERCI_CORTEX_ROOT=%PERCI_ROOT%\Cortex"',
        'set "PERCI_CORTEX_HOME=%PERCI_ROOT%\.perci\cortex-home"',
        'set "PERCI_CORTEX_PYTHON=%PERCI_ROOT%\Cortex\.venv\Scripts\python.exe"',
        'set "CORTEX_HOME=%PERCI_ROOT%\.perci\cortex-home"',
        'set "PERCI_CORTEX_REPO=Perci"',
        'rem Perci resolves models\ and memory\ relative to the repo, so run from there.',
        'pushd "%PERCI_ROOT%"',
        'if "%~1"=="" goto chat',
        '"%PERCI_EXE%" %*',
        'goto done',
        ':chat',
        '"%PERCI_EXE%" chat',
        ':done',
        'set "PERCI_RC=%ERRORLEVEL%"',
        'popd',
        'endlocal & exit /b %PERCI_RC%',
        ':rebuild',
        'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%PERCI_ROOT%\scripts\Install-PerciCommand.ps1" -Rebuild',
        'endlocal & exit /b %ERRORLEVEL%',
        ':notbuilt',
        'echo Perci isn''t built yet. Run: powershell -NoProfile -ExecutionPolicy Bypass -File "%PERCI_ROOT%\scripts\Install-PerciCommand.ps1"',
        'endlocal & exit /b 1'
    )
    [System.IO.File]::WriteAllText($Shim, (($lines -join "`r`n") + "`r`n"), [System.Text.Encoding]::ASCII)
    Write-Ok "command shim $Shim"
}

function Add-BinDirToUserPath {
    # Read/write HKCU\Environment directly so existing %VAR% entries stay
    # unexpanded (REG_EXPAND_SZ). No admin needed.
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $userPath = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $norm = { param($p) ([Environment]::ExpandEnvironmentVariables($p)).Trim().TrimEnd('\').ToLowerInvariant() }
        $want = & $norm $BinDir
        $parts = @($userPath -split ';' | Where-Object { $_.Trim() -ne '' })
        $present = $false
        foreach ($p in $parts) {
            if ((& $norm $p) -eq $want) { $present = $true }
        }
        if ($present) {
            Write-Ok "$BinDir is already on your user PATH"
        } else {
            $newPath = (@($parts) + $BinDir) -join ';'
            $key.SetValue('Path', $newPath, [Microsoft.Win32.RegistryValueKind]::ExpandString)
            Write-Ok "added $BinDir to your user PATH"
        }
    } finally {
        $key.Close()
    }
    # Setting a user variable through .NET broadcasts WM_SETTINGCHANGE, so
    # terminals opened from now on pick up the new PATH. PERCI_HOME also
    # records where this clone lives.
    [Environment]::SetEnvironmentVariable('PERCI_HOME', $Root, 'User')
    if ((($env:Path -split ';') | ForEach-Object { & $norm $_ }) -notcontains $want) {
        $env:Path = "$env:Path;$BinDir"
    }
}

function Test-PerciCommandConflicts {
    $others = @(Get-Command perci -All -ErrorAction SilentlyContinue |
        Where-Object { $_.Source -and ($_.Source -ine $Shim) })
    foreach ($o in $others) {
        Write-Note "Another 'perci' is also on PATH and may win: $($o.Source)"
    }
}

function Set-PerciDesktopShortcut {
    try {
        $launcher = Join-Path $Root 'Start-Perci.cmd'
        if (-not (Test-Path -LiteralPath $launcher)) { return }
        $desk = [Environment]::GetFolderPath('Desktop')
        if (-not $desk -or -not (Test-Path -LiteralPath $desk)) { return }
        $shell = New-Object -ComObject WScript.Shell
        $lnk = $shell.CreateShortcut((Join-Path $desk 'Perci.lnk'))
        $lnk.TargetPath = $launcher
        $lnk.Arguments = ''
        $lnk.WorkingDirectory = $Root
        $lnk.Description = 'Perci - local assistant'
        $ico = Join-Path $Root 'assets\icons\perci-darkblood.ico'
        if (Test-Path -LiteralPath $ico) { $lnk.IconLocation = "$ico,0" }
        $lnk.Save()
        Write-Ok "Desktop shortcut $(Join-Path $desk 'Perci.lnk')"
    } catch {
        Write-Note "Skipped Desktop shortcut ($($_.Exception.Message)). The perci command still works."
    }
}

Write-Host ''
Write-Host "  Perci installer  ($Root)" -ForegroundColor White
Write-Host ''

if ($Rebuild) {
    Invoke-PerciReleaseBuild
    exit 0
}

Test-PerciWeights
Invoke-PerciReleaseBuild
Write-PerciShim
Add-BinDirToUserPath
Test-PerciCommandConflicts
if (-not $NoShortcut) { Set-PerciDesktopShortcut }

Write-Host ''
Write-Host '  Done. Next steps:' -ForegroundColor White
Write-Host '    1. Open a NEW terminal window (PowerShell, cmd, or Windows Terminal).'
Write-Host '    2. Type:  perci            -> opens chat'
Write-Host '              perci ask "hi"   -> one-shot answer'
Write-Host '              perci --rebuild  -> rebuild after pulling new code'
Write-Host ''
Write-Host '  (This window can use it right now too; other already-open windows need a restart.)' -ForegroundColor DarkGray
Write-Host ''

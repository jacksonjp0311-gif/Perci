[CmdletBinding()]
param(
    [ValidateSet('chat', 'status', 'test', 'bench', 'intel', 'cortex-init', 'rebuild')]
    [string]$Mode = 'chat'
)

# Perci launcher (Desktop shortcut / Start-Perci.cmd).
# Runs the already-built release binary directly: no cargo step at launch.
# Building happens only in scripts\Install-PerciCommand.ps1
# (or `perci --rebuild` / `Launch-Perci.ps1 -Mode rebuild`).

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Installer = Join-Path $Root 'scripts\Install-PerciCommand.ps1'

function Write-PerciLine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text,
        [ConsoleColor]$Color = [ConsoleColor]::Gray
    )
    Write-Host $Text -ForegroundColor $Color
}

function Repair-PerciDesktopShortcut {
    <#
    .SYNOPSIS
      Best-effort: keep an existing Desktop Perci.lnk pointed at this clone's
      Start-Perci.cmd (fixes shortcuts left over from an older folder) and use
      assets/icons/perci-darkblood.ico when it exists. Never blocks launch.
    #>
    $ico = Join-Path $Root 'assets\icons\perci-darkblood.ico'
    $launcher = Join-Path $Root 'Start-Perci.cmd'
    $haveIco = Test-Path -LiteralPath $ico
    $desktopDirs = @(
        (Join-Path $env:USERPROFILE 'OneDrive\Desktop'),
        (Join-Path $env:USERPROFILE 'Desktop')
    ) | Where-Object { Test-Path -LiteralPath $_ }
    foreach ($desk in $desktopDirs) {
        $lnkPath = Join-Path $desk 'Perci.lnk'
        if (-not (Test-Path -LiteralPath $lnkPath)) {
            continue
        }
        try {
            $shell = New-Object -ComObject WScript.Shell
            $lnk = $shell.CreateShortcut($lnkPath)
            $changed = $false
            $targetOk = $lnk.TargetPath -and (Test-Path -LiteralPath $lnk.TargetPath)
            $argsPointHere = ($lnk.Arguments -like "*$Root*") -or ($lnk.TargetPath -ieq $launcher)
            if (-not $targetOk -or -not $argsPointHere) {
                $lnk.TargetPath = $launcher
                $lnk.Arguments = ''
                $lnk.WorkingDirectory = $Root
                $changed = $true
            }
            if ($haveIco) {
                $want = "$ico,0"
                if ($lnk.IconLocation -ne $want) {
                    $lnk.IconLocation = $want
                    $changed = $true
                }
            }
            if ($changed) {
                $lnk.Description = 'Perci - local assistant'
                $lnk.Save()
            }
        } catch {
            # Best-effort; never block chat launch on shortcut repair.
        }
    }
}

Push-Location $Root

try {
    try { $Host.UI.RawUI.WindowTitle = 'Perci' } catch {}

    if ($Mode -eq 'rebuild') {
        & $Installer -Rebuild
        exit $LASTEXITCODE
    }

    $env:PYTHONUTF8 = '1'
    $env:PERCI_CORTEX_MODE = if ($env:PERCI_CORTEX_MODE) { $env:PERCI_CORTEX_MODE } else { 'auto' }

    $V3Weights = Join-Path $Root 'models\perci-cognitive-v0.3.pwgt'
    $V2Weights = Join-Path $Root 'models\perci-cognitive-v0.2.pwgt'
    $V1Weights = Join-Path $Root 'models\perci-cognitive-v0.1.pwgt'
    $Weights = if ($env:PERCI_WEIGHTS) {
        $env:PERCI_WEIGHTS
    } elseif (Test-Path -LiteralPath $V3Weights -PathType Leaf) {
        $V3Weights
    } elseif (Test-Path -LiteralPath $V2Weights -PathType Leaf) {
        $V2Weights
    } else {
        $V1Weights
    }
    if (-not (Test-Path -LiteralPath $Weights -PathType Leaf)) {
        throw "Perci weights are missing: $Weights"
    }

    $Stream = [System.IO.File]::OpenRead($Weights)
    try {
        $MagicBytes = New-Object byte[] 8
        if ($Stream.Read($MagicBytes, 0, 8) -ne 8) {
            throw "Perci weights are truncated: $Weights"
        }
        $Magic = [Text.Encoding]::ASCII.GetString($MagicBytes)
    } finally {
        $Stream.Dispose()
    }
    if ($Magic -notin @('PERCIW01', 'PERCIW02', 'PERCIW03')) {
        if ($Magic -like 'version*') {
            throw "Perci weights are a Git LFS pointer, not the real file. Run: git lfs install; git lfs pull   (in $Root)"
        }
        throw "Perci weights have an unknown signature '$Magic': $Weights"
    }
    $env:PERCI_WEIGHTS = $Weights

    $CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    $CargoExe = Join-Path $CargoBin 'cargo.exe'
    if ((Test-Path -LiteralPath $CargoExe) -and (($env:Path -split ';') -notcontains $CargoBin)) {
        $env:Path = "$CargoBin;$env:Path"
    }

    $CortexRoot = Join-Path $Root 'Cortex'
    $CortexHome = Join-Path $Root '.perci\cortex-home'
    $CortexPython = Join-Path $CortexRoot '.venv\Scripts\python.exe'
    $env:PERCI_CORTEX_ROOT = $CortexRoot
    $env:PERCI_CORTEX_HOME = $CortexHome
    $env:PERCI_CORTEX_PYTHON = $CortexPython
    $env:CORTEX_HOME = $CortexHome
    $env:PERCI_CORTEX_REPO = 'Perci'

    if ($Mode -eq 'cortex-init') {
        & (Join-Path $Root 'Initialize-Perci-Cortex.ps1')
        exit $LASTEXITCODE
    }

    # Repair a stale Desktop shortcut without blocking launch.
    Repair-PerciDesktopShortcut

    if ($Mode -eq 'test') {
        # Explicit developer mode only; normal launches never invoke cargo.
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            throw 'Rust/Cargo is required for -Mode test.'
        }
        & cargo test --release
        exit $LASTEXITCODE
    }

    # The installer builds into target\release (PERCI_TARGET_DIR overrides the target root).
    $TargetRoot = if ($env:PERCI_TARGET_DIR) { $env:PERCI_TARGET_DIR } else { Join-Path $Root 'target' }
    $Exe = Join-Path $TargetRoot 'release\perci.exe'
    if (-not (Test-Path -LiteralPath $Exe)) {
        Write-PerciLine ("  Perci isn't built yet. Run: powershell -NoProfile -ExecutionPolicy Bypass -File `"{0}`"" -f $Installer) Yellow
        if (-not [Console]::IsInputRedirected) {
            [void](Read-Host '  Press Enter to close')
        }
        exit 1
    }

    if ($Mode -eq 'status') {
        Clear-Host
        & $Exe status
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'bench') {
        & $Exe bench
        exit $LASTEXITCODE
    }

    if ($Mode -eq 'intel') {
        Clear-Host
        & $Exe intel
        exit $LASTEXITCODE
    }

    $env:PERCI_COLOR = if ($env:PERCI_COLOR) { $env:PERCI_COLOR } else { 'always' }
    # Wipe the PowerShell preamble so the Perci welcome header sits at the top.
    Clear-Host
    & $Exe chat
    exit $LASTEXITCODE
}
catch {
    # Do not let an exception make a double-clicked console vanish.
    # Preserve the exact error for diagnosis and keep the message visible when
    # launched interactively; piped/automated launches still return exit 1.
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $errorPath = Join-Path ([System.IO.Path]::GetTempPath()) ("perci-launch-error-{0}.log" -f $stamp)
    $errorText = ($_ | Out-String).Trim()
    try {
        Set-Content -LiteralPath $errorPath -Value $errorText -Encoding utf8
    } catch {}
    try {
        Write-Host ''
        Write-PerciLine '  Perci could not start.' Red
        Write-PerciLine ("  {0}" -f $_.Exception.Message) Gray
        Write-PerciLine ("  Error log: {0}" -f $errorPath) DarkGray
        if (-not [Console]::IsInputRedirected) {
            [void](Read-Host '  Press Enter to close')
        }
    } catch {}
    exit 1
}
finally {
    Pop-Location
}

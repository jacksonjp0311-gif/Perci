[CmdletBinding()]
param(
    [ValidateSet('chat', 'status', 'test', 'bench', 'intel', 'cortex-init')]
    [string]$Mode = 'chat'
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path

function Write-BloodLine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text,
        [ConsoleColor]$Color = [ConsoleColor]::DarkRed
    )
    Write-Host $Text -ForegroundColor $Color
}

function Get-RedDnaFrames {
    # ASCII double-helix (encoding-safe on Windows PowerShell).
    # Backbone = o, rungs = -, phase shifts each frame.
    @(
        @(
            '      o-----o',
            '     /       \',
            '    o         o',
            '     \       /',
            '      o-----o',
            '     /       \',
            '    o         o',
            '     \       /',
            '      o-----o'
        ),
        @(
            '     o-----o',
            '    /       \',
            '   o         o',
            '    \       /',
            '     o-----o',
            '    /       \',
            '   o         o',
            '    \       /',
            '     o-----o'
        ),
        @(
            '    o-----o',
            '   /       \',
            '  o         o',
            '   \       /',
            '    o-----o',
            '   /       \',
            '  o         o',
            '   \       /',
            '    o-----o'
        ),
        @(
            '   o-----o',
            '  /       \',
            ' o         o',
            '  \       /',
            '   o-----o',
            '  /       \',
            ' o         o',
            '  \       /',
            '   o-----o'
        ),
        @(
            '  o-----o',
            ' /       \',
            'o         o',
            ' \       /',
            '  o-----o',
            ' /       \',
            'o         o',
            ' \       /',
            '  o-----o'
        ),
        @(
            ' o-----o',
            '/       \',
            'o       o',
            '\       /',
            ' o-----o',
            '/       \',
            'o       o',
            '\       /',
            ' o-----o'
        ),
        @(
            'o-----o',
            '       \',
            'o       o',
            '       /',
            'o-----o',
            '       \',
            'o       o',
            '       /',
            'o-----o'
        ),
        @(
            ' o-----o',
            '/       \',
            'o       o',
            '\       /',
            ' o-----o',
            '/       \',
            'o       o',
            '\       /',
            ' o-----o'
        )
    )
}

function Show-RedDnaBanner {
    param([string]$VersionLabel = 'Perci')
    Write-Host ''
    Write-BloodLine '  +------------------------------------------+' DarkRed
    Write-BloodLine '  |            *                             |' Red
    Write-BloodLine '  |           / \     RED DNA SYNC           |' DarkRed
    Write-BloodLine '  |          *   *    dark-blood runtime     |' Red
    Write-BloodLine '  |           \ /                            |' DarkRed
    Write-BloodLine '  |            *                             |' Red
    $label = $VersionLabel
    if ($label.Length -gt 38) { $label = $label.Substring(0, 38) }
    $pad = 38 - $label.Length
    Write-BloodLine ("  |  {0}{1} |" -f $label, (' ' * $pad)) DarkRed
    Write-BloodLine '  +------------------------------------------+' DarkRed
    Write-Host ''
}

function Sync-PerciRuntimeWithRedDna {
    <#
    .SYNOPSIS
      Run cargo build while a red DNA helix animation plays (dark-blood theme).
    #>
    param(
        [string[]]$CargoArgs = @('build', '--release')
    )

    $verLabel = 'Perci'
    try {
        $toml = Get-Content (Join-Path $Root 'Cargo.toml') -Raw
        if ($toml -match 'version\s*=\s*"([^"]+)"') {
            $verLabel = "Perci v$($Matches[1])"
        }
    } catch {}

    $frames = Get-RedDnaFrames
    $stamp = [guid]::NewGuid().ToString('N').Substring(0, 8)
    $logPath = Join-Path ([System.IO.Path]::GetTempPath()) ("perci-dna-build-{0}.log" -f $stamp)
    $outPath = Join-Path ([System.IO.Path]::GetTempPath()) ("perci-dna-out-{0}.log" -f $stamp)
    $errPath = Join-Path ([System.IO.Path]::GetTempPath()) ("perci-dna-err-{0}.log" -f $stamp)

    $cargoCmd = (Get-Command cargo -ErrorAction SilentlyContinue).Source
    if (-not $cargoCmd) {
        throw 'Rust/Cargo is required for first build or source updates.'
    }

    $argLine = ($CargoArgs -join ' ')
    $proc = Start-Process -FilePath $cargoCmd `
        -ArgumentList $argLine `
        -WorkingDirectory $Root `
        -WindowStyle Hidden `
        -RedirectStandardOutput $outPath `
        -RedirectStandardError $errPath `
        -PassThru

    $interactive = $false
    try {
        $interactive = -not [Console]::IsOutputRedirected -and $Host.Name -ne 'ServerRemoteHost'
        if ($interactive) {
            $null = [Console]::CursorTop
        }
    } catch {
        $interactive = $false
    }

    try {
        if ($interactive) {
            try { Clear-Host } catch {}
            Show-RedDnaBanner -VersionLabel $verLabel
            Write-BloodLine '  *  Synchronizing optimized Perci runtime...' Red
            Write-BloodLine '  .  sparse strands | integer hot path | governed promote' DarkRed
            Write-Host ''
        } else {
            Write-BloodLine '  *  Synchronizing optimized Perci runtime (red-dna)...' Red
        }

        $frameIdx = 0
        $tick = 0
        $phases = @(
            'unzipping base pairs',
            'aligning Bitwork strands',
            'annealing release symbols',
            'ligating native fields',
            'proofreading gates',
            'coiling into live target'
        )

        $statusLines = 2
        $helixLines = $frames[0].Count
        $block = $helixLines + $statusLines
        $cursorTop = 0
        if ($interactive) {
            for ($i = 0; $i -lt $block; $i++) {
                Write-Host ''
            }
            try {
                $cursorTop = [Console]::CursorTop - $block
                if ($cursorTop -lt 0) { $cursorTop = 0 }
            } catch {
                $interactive = $false
            }
        }

        while (-not $proc.HasExited) {
            $frame = $frames[$frameIdx % $frames.Count]
            $phase = $phases[$tick % $phases.Count]
            $dots = 1 + ($tick % 4)
            $pulse = ('*' * $dots) + ('.' * (4 - $dots))

            if ($interactive) {
                try {
                    [Console]::SetCursorPosition(0, $cursorTop)
                    foreach ($line in $frame) {
                        Write-Host ('  {0}' -f $line.PadRight(28)) -ForegroundColor Red
                    }
                    Write-Host ('  {0}  {1}' -f $pulse, $phase.PadRight(36)) -ForegroundColor DarkRed
                    Write-Host '  red-dna // dark-blood compile path'.PadRight(50) -ForegroundColor DarkRed
                } catch {
                    $interactive = $false
                    Write-BloodLine ("  *  red-dna: {0}" -f $phase) DarkRed
                }
            } elseif (($tick % 8) -eq 0) {
                Write-BloodLine ("  *  red-dna: {0}" -f $phase) DarkRed
            }

            $frameIdx++
            $tick++
            Start-Sleep -Milliseconds 90
        }

        if (-not $proc.HasExited) {
            $proc.WaitForExit()
        }
        # Start-Process can leave ExitCode null for a moment after HasExited flips.
        for ($i = 0; $i -lt 40 -and $null -eq $proc.ExitCode; $i++) {
            Start-Sleep -Milliseconds 25
            try { $proc.Refresh() } catch {}
        }
        $code = $proc.ExitCode

        $outText = ''
        $errText = ''
        if (Test-Path -LiteralPath $outPath) {
            $outText = Get-Content -LiteralPath $outPath -Raw -ErrorAction SilentlyContinue
        }
        if (Test-Path -LiteralPath $errPath) {
            $errText = Get-Content -LiteralPath $errPath -Raw -ErrorAction SilentlyContinue
        }
        $combined = (("$outText`n$errText").Trim())
        if ($combined) {
            Set-Content -LiteralPath $logPath -Value $combined -Encoding utf8
        }

        if ($null -eq $code) {
            if ($combined -match 'Finished [`'']?release[`'']? profile') {
                $code = 0
            } elseif ($combined -match '(?i)error:|could not compile|build failed') {
                $code = 1
            } else {
                $code = 1
            }
        }

        if ($code -ne 0) {
            Write-Host ''
            Write-BloodLine '  *  DNA sync FAILED - cargo could not anneal the strand.' Red
            if ($combined) {
                Write-Host $combined -ForegroundColor DarkYellow
            }
            throw "Perci release build failed (exit $code). Exit any other live Perci window and retry. Log: $logPath"
        }

        Write-Host ''
        Write-BloodLine '  *  DNA sync complete - runtime strand sealed.' Red
        if ($interactive) {
            Write-BloodLine '  .  clearing console -> dark-blood chat' DarkRed
            Start-Sleep -Milliseconds 220
        }
    }
    finally {
        if ($proc -and -not $proc.HasExited) {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
        }
        Remove-Item -LiteralPath $outPath, $errPath -Force -ErrorAction SilentlyContinue
    }
}

Push-Location $Root

try {
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

    $Cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $Cargo) {
        throw 'Rust/Cargo is required for first build or source updates.'
    }

    if ($Mode -eq 'test') {
        & cargo test --release
        exit $LASTEXITCODE
    }

    # Always let Cargo verify source freshness. Timestamp-only checks can select
    # a stale executable after source files are copied, restored, or synced.
    $LiveTarget = if ($env:PERCI_TARGET_DIR) {
        $env:PERCI_TARGET_DIR
    } else {
        Join-Path $Root 'target\live'
    }
    $env:CARGO_TARGET_DIR = $LiveTarget
    $Exe = Join-Path $LiveTarget 'release\perci.exe'

    # Dark-blood red DNA sync: helix animates while cargo builds in background.
    # Cleared before chat so the banner still snaps to the top.
    Sync-PerciRuntimeWithRedDna -CargoArgs @('build', '--release')

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

    # Fade out preamble: wipe PS copyright + cargo lines so Perci snaps to top.
    $ver = 'Perci'
    try {
        $toml = Get-Content (Join-Path $Root 'Cargo.toml') -Raw
        if ($toml -match 'version\s*=\s*"([^"]+)"') { $ver = "Perci v$($Matches[1])" }
    } catch {}
    $Host.UI.RawUI.WindowTitle = "$ver // dark-blood"
    $env:PERCI_COLOR = if ($env:PERCI_COLOR) { $env:PERCI_COLOR } else { 'always' }
    Clear-Host
    # Soft beat so clear is perceived as a transition, then chat paints the banner.
    Start-Sleep -Milliseconds 80
    & $Exe chat
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}

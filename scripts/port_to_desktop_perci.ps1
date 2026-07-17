# Port LumenShell/perci advancements to Desktop/perci (original tree).
#
# Default: source, knowledge packs, scripts, docs, config - NOT weights/target.
# Use -Weights to copy the promoted v2 pack (or legacy v1 fallback), with backup.
#
# Usage:
#   .\scripts\port_to_desktop_perci.ps1
#   .\scripts\port_to_desktop_perci.ps1 -Weights
#   .\scripts\port_to_desktop_perci.ps1 -Dest "C:\Users\jacks\OneDrive\Desktop\perci" -WhatIf

param(
    [string]$Source = "",
    [string]$Dest = "C:\Users\jacks\OneDrive\Desktop\perci",
    [switch]$Weights,
    [switch]$WhatIf
)

$ErrorActionPreference = "Stop"

if (-not $Source) {
    $Source = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

if (-not (Test-Path $Source)) { throw "Source missing: $Source" }
if (-not (Test-Path $Dest)) { throw "Dest missing: $Dest - create or clone Desktop perci first" }

Write-Host "Port Perci advancements" -ForegroundColor Cyan
Write-Host "  from: $Source"
Write-Host "  to:   $Dest"
if ($WhatIf) { Write-Host "  mode: WhatIf (no writes)" -ForegroundColor Yellow }

function Copy-Tree($rel) {
    $from = Join-Path $Source $rel
    $to = Join-Path $Dest $rel
    if (-not (Test-Path $from)) {
        Write-Host "  skip missing $rel" -ForegroundColor DarkGray
        return
    }
    if ($WhatIf) {
        Write-Host "  would copy $rel" -ForegroundColor DarkYellow
        return
    }
    $parent = Split-Path $to -Parent
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    if (Test-Path $from -PathType Container) {
        robocopy $from $to /E /XD target .git __pycache__ .venv /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
        if ($LASTEXITCODE -ge 8) { throw "robocopy failed for $rel code=$LASTEXITCODE" }
    } else {
        Copy-Item $from $to -Force
    }
    Write-Host "  + $rel" -ForegroundColor Green
}

Copy-Tree "src"
Copy-Tree "Cargo.toml"
if (Test-Path (Join-Path $Source "Cargo.lock")) { Copy-Tree "Cargo.lock" }

Copy-Tree "knowledge"
Copy-Tree "scripts"
Copy-Tree "docs"
Copy-Tree "config"
Copy-Tree "training\adaptive"
Copy-Tree "training\curriculum"
Copy-Tree "training\from-lumen"
Copy-Tree "training\README.md"
Copy-Tree "training\heldout-v2.jsonl"
Copy-Tree "training\heldout-v2.1.jsonl"
Copy-Tree "WEIGHTS.md"
Copy-Tree "VALIDATION.md"
if (Test-Path (Join-Path $Source "AGENTS.md")) { Copy-Tree "AGENTS.md" }
if (Test-Path (Join-Path $Source "README.md")) { Copy-Tree "README.md" }

if ($Weights) {
    $modelName = if (Test-Path (Join-Path $Source "models\perci-cognitive-v0.2.pwgt")) {
        "perci-cognitive-v0.2.pwgt"
    } else {
        "perci-cognitive-v0.1.pwgt"
    }
    $wFrom = Join-Path $Source "models\$modelName"
    $wTo = Join-Path $Dest "models\$modelName"
    if (Test-Path $wFrom) {
        if (-not $WhatIf) {
            New-Item -ItemType Directory -Force -Path (Join-Path $Dest "models") | Out-Null
            if (Test-Path $wTo) {
                $bak = "$wTo.bak-port-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
                Copy-Item $wTo $bak -Force
                Write-Host "  backup dest weights -> $bak" -ForegroundColor Yellow
            }
            Copy-Item $wFrom $wTo -Force
            $jFrom = "$wFrom.json"
            if (Test-Path $jFrom) { Copy-Item $jFrom "$wTo.json" -Force }
        }
        Write-Host "  + models/$modelName (+ json)" -ForegroundColor Green
        if ($modelName -like '*v0.2*') {
            Copy-Tree "models\promotion-ledger.jsonl"
            Copy-Tree "models\candidates\evaluation-v2.1.3-operational.json"
        }
    }
} else {
    Write-Host "  (weights skipped - pass -Weights to copy morphed .pwgt)" -ForegroundColor DarkGray
}

Write-Host ""
Write-Host "Next on Desktop/perci:" -ForegroundColor Cyan
Write-Host "  cd $Dest"
Write-Host "  cargo build --release"
Write-Host "  .\target\release\perci.exe status"
Write-Host "  .\target\release\perci.exe intel"
Write-Host "  python scripts\adaptive_train.py"
if (-not $Weights) {
    Write-Host "  # optional: re-run port with -Weights after morph, or morph there"
}
Write-Host "Done." -ForegroundColor Green

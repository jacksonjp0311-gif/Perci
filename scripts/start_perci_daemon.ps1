# Start long-lived Perci daemon (warm weights, multi-ask without process spawn).
# Default: 127.0.0.1:17865
param(
    [switch]$Stop,
    [switch]$Foreground
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Bin = Join-Path $Root "target\release\perci.exe"
if (-not (Test-Path $Bin)) {
    Write-Host "Building perci release..." -ForegroundColor Cyan
    Push-Location $Root
    cargo build --release
    Pop-Location
}
if (-not (Test-Path $Bin)) { throw "perci.exe missing after build" }

$v3 = Join-Path $Root "models\perci-cognitive-v0.3.pwgt"
$v2 = Join-Path $Root "models\perci-cognitive-v0.2.pwgt"
$v1 = Join-Path $Root "models\perci-cognitive-v0.1.pwgt"
$env:PERCI_WEIGHTS = if (Test-Path $v3) { $v3 } elseif (Test-Path $v2) { $v2 } else { $v1 }
$env:PERCI_PACKS = Join-Path $Root "knowledge\packs"
$env:PERCI_MEMORY = Join-Path $Root "memory\perci.jsonl"
$env:PERCI_SESSION = Join-Path $Root "memory\session.jsonl"
if (-not $env:PERCI_DAEMON_PORT) { $env:PERCI_DAEMON_PORT = "17865" }
$port = [int]$env:PERCI_DAEMON_PORT

if ($Stop) {
    try {
        $c = New-Object System.Net.Sockets.TcpClient("127.0.0.1", $port)
        $s = $c.GetStream()
        $bytes = [Text.Encoding]::UTF8.GetBytes("{`"op`":`"shutdown`"}`n")
        $s.Write($bytes, 0, $bytes.Length)
        $c.Close()
        Write-Host "shutdown sent" -ForegroundColor Yellow
    } catch {
        Write-Host "daemon not reachable" -ForegroundColor DarkGray
    }
    exit 0
}

try {
    $c = New-Object System.Net.Sockets.TcpClient("127.0.0.1", $port)
    $c.Close()
    Write-Host "perci daemon already live on port $port" -ForegroundColor Green
    exit 0
} catch { }

if ($Foreground) {
    Write-Host "perci daemon foreground port $port" -ForegroundColor Cyan
    Set-Location $Root
    & $Bin daemon
    exit $LASTEXITCODE
}

$mem = Join-Path $Root "memory"
New-Item -ItemType Directory -Force -Path $mem | Out-Null
$logOut = Join-Path $mem "daemon.out.log"
$logErr = Join-Path $mem "daemon.err.log"

$proc = Start-Process -FilePath $Bin -ArgumentList "daemon" -WorkingDirectory $Root `
    -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $logOut `
    -RedirectStandardError $logErr

Start-Sleep -Milliseconds 800
$live = $false
try {
    $c = New-Object System.Net.Sockets.TcpClient("127.0.0.1", $port)
    $c.Close()
    $live = $true
} catch { }

if ($live) {
    Write-Host "perci daemon started pid=$($proc.Id) port=$port" -ForegroundColor Green
} else {
    Write-Host "daemon pid=$($proc.Id) but port not open - check logs" -ForegroundColor Yellow
}
Write-Host "  out: $logOut"
Write-Host "  err: $logErr"
Write-Host "  stop: .\scripts\start_perci_daemon.ps1 -Stop"

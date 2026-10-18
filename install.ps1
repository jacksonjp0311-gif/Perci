# Perci installer for Windows (x64; ARM64 runs it under emulation).
#
#   irm https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.ps1 | iex
#   $env:PERCI_INSTALL_WEIGHTS = "tools"; irm .../install.ps1 | iex     # tool layer + route pack only (~20 MB)
#   .\install.ps1 -Weights tools -Version v0.11.8               # when saved to a file
#
# Downloads the release zip, verifies it against the release's SHA256SUMS,
# unpacks it into %LOCALAPPDATA%\Perci\app (a complete Perci home), adds its
# bin folder to your user PATH, downloads + verifies the weights with
# `perci fetch-weights`, and runs `perci doctor`.
#
# Parameters (or environment variables):
#   -Version vX.Y.Z   PERCI_VERSION        release tag (default: the newest release, previews included)
#   -Dir DIR          PERCI_INSTALL_DIR    install folder
#   -Weights SET      PERCI_INSTALL_WEIGHTS        default (~240 MB) | tools (~20 MB) | all | none
#   -NoModifyPath     PERCI_NO_MODIFY_PATH=1
#   PERCI_RELEASE_URL download base for the release assets (mirrors, testing)
#
# Re-running upgrades in place; weights that still match the manifest are kept.

function Install-Perci {
    param(
        [string]$Version = $(if ($env:PERCI_VERSION) { $env:PERCI_VERSION } else { "latest" }),
        [string]$Dir = $(if ($env:PERCI_INSTALL_DIR) { $env:PERCI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Perci\app" }),
        [ValidateSet("default", "tools", "all", "none")]
        [string]$Weights = $(if ($env:PERCI_INSTALL_WEIGHTS) { $env:PERCI_INSTALL_WEIGHTS } else { "default" }),
        [switch]$NoModifyPath
    )
    $ErrorActionPreference = "Stop"
    $ProgressPreference = "SilentlyContinue"
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    if ($env:PERCI_NO_MODIFY_PATH -eq "1") { $NoModifyPath = $true }

    $repo = "jacksonjp0311-gif/Perci"
    $target = "x86_64-pc-windows-msvc"
    if ($Version -eq "latest" -and -not $env:PERCI_RELEASE_URL) {
        # Newest release including previews (/releases/latest skips prereleases).
        try {
            $rel = Invoke-RestMethod -UseBasicParsing -Uri "https://api.github.com/repos/$repo/releases?per_page=1"
            $tag = @($rel)[0].tag_name
            if ($tag) { $Version = $tag }
        } catch { }
    }
    if ($env:PERCI_RELEASE_URL) {
        $base = $env:PERCI_RELEASE_URL.TrimEnd("/")
    } elseif ($Version -eq "latest") {
        $base = "https://github.com/$repo/releases/latest/download"
    } else {
        if (-not $Version.StartsWith("v")) { $Version = "v$Version" }
        $base = "https://github.com/$repo/releases/download/$Version"
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("perci-install-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $asset = "perci-$target.zip"
        Write-Host "perci install: downloading $asset from $base"
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile (Join-Path $tmp $asset)
        Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile (Join-Path $tmp "SHA256SUMS")
        $want = $null
        foreach ($line in Get-Content (Join-Path $tmp "SHA256SUMS")) {
            $parts = $line -split "\s+", 2
            if ($parts.Count -eq 2 -and $parts[1].TrimStart("*") -eq $asset) { $want = $parts[0].ToLowerInvariant() }
        }
        if (-not $want) { throw "perci install: $asset is not listed in SHA256SUMS" }
        $got = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $asset)).Hash.ToLowerInvariant()
        if ($want -ne $got) { throw "perci install: checksum mismatch for $asset (expected $want, got $got)" }
        Write-Host "perci install: sha256 ok ($got)"

        $x = Join-Path $tmp "x"
        Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $x -Force
        $src = Join-Path $x "perci"
        if (-not (Test-Path (Join-Path $src "bin\perci.exe"))) { throw "perci install: archive has no perci\bin\perci.exe" }
        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
        # Copy over the previous install; weights already in models\ are kept.
        Copy-Item -Path (Join-Path $src "*") -Destination $Dir -Recurse -Force
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }

    $bin = Join-Path $Dir "bin"
    $perci = Join-Path $bin "perci.exe"
    Write-Host ("perci install: installed " + (& $perci --version) + " to $Dir")

    if (-not $NoModifyPath) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $entries = @($userPath -split ";" | Where-Object { $_ })
        if ($entries -notcontains $bin) {
            [Environment]::SetEnvironmentVariable("Path", (($entries + $bin) -join ";"), "User")
            Write-Host "perci install: added $bin to your user PATH (new terminals pick it up)"
        }
    }
    if (($env:Path -split ";") -notcontains $bin) { $env:Path = "$bin;$env:Path" }

    $env:PERCI_HOME = $Dir
    try {
        if ($Weights -ne "none") {
            if ($env:PERCI_RELEASE_URL -and -not $env:PERCI_WEIGHTS_URL) { $env:PERCI_WEIGHTS_URL = $base }
            Write-Host "perci install: fetching weights (set: $Weights)"
            & $perci fetch-weights --set $Weights
            if ($LASTEXITCODE -ne 0) { throw "perci install: weights download failed; re-run: perci fetch-weights --set $Weights" }
        }
        if ($Weights -eq "default" -or $Weights -eq "all") { & $perci doctor } else { & $perci doctor --tools }
        if ($LASTEXITCODE -ne 0) { throw "perci install: perci doctor reported a problem (see above)" }
    } finally {
        Remove-Item Env:\PERCI_HOME -ErrorAction SilentlyContinue
    }

    Write-Host ""
    Write-Host "Perci is installed. In a new terminal:"
    Write-Host '  ''{"expr":"17% of 240","claimed":"41"}'' | perci tool exact'
    Write-Host "  claude mcp add perci -- perci mcp          # Claude Code; other harnesses: docs\QUICKSTART.md"
    Write-Host "Quickstart: $Dir\docs\QUICKSTART.md"
}

Install-Perci @args

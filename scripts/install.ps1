# cgen installer for Windows
# Usage: irm https://raw.githubusercontent.com/gtkacz/rust-auto-commit/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "gtkacz/rust-auto-commit"
$BinaryName = "cgen.exe"
$Artifact = "cgen-windows-amd64.exe"

function Write-Info($msg) { Write-Host $msg -ForegroundColor Cyan }
function Write-Success($msg) { Write-Host $msg -ForegroundColor Green }
function Write-Err($msg) { Write-Host "error: $msg" -ForegroundColor Red; exit 1 }

# Get latest release tag
Write-Info "Fetching latest release..."
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    $Version = $Release.tag_name
} catch {
    Write-Err "Could not fetch latest release. Check https://github.com/$Repo/releases"
}

Write-Info "Latest version: $Version"

$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$Artifact"

# Determine install directory
$InstallDir = Join-Path $env:LOCALAPPDATA "cgen"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$InstallPath = Join-Path $InstallDir $BinaryName

# Download
Write-Info "Downloading $Artifact..."
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $InstallPath -UseBasicParsing
} catch {
    Write-Err "Download failed. URL: $DownloadUrl"
}

# Add to PATH if not already there
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Info "Adding $InstallDir to user PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

Write-Success "`ncgen $Version installed successfully!"
Write-Host ""
Write-Host "  Installed to: $InstallPath"
Write-Host "  Run 'cgen config' to set up your API key."
Write-Host "  Run 'cgen --help' for usage information."
Write-Host ""
Write-Host "  Restart your terminal for PATH changes to take effect." -ForegroundColor Yellow

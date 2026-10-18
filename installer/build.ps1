#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Local developer convenience wrapper for building the Ace Blocker MSI.

.DESCRIPTION
    Builds the engine/watchdog Cargo workspace packages and the Tauri app,
    stages the exact inputs
    required by package.ps1 in a temporary directory, then invokes the
    source-layout-agnostic packager.

    CI may call package.ps1 with explicitly built workspace artifacts.
#>
param(
    [string]$Version,
    [string]$OutputDir
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$EngineDir = Join-Path $ScriptDir "..\crates\ace-engine"
$AppDir = Join-Path $ScriptDir "..\apps\ace-app"
$PackageScript = Join-Path $ScriptDir "package.ps1"

if (-not (Test-Path -LiteralPath $EngineDir)) {
    throw "Local ace-engine checkout not found at '$EngineDir'. Use package.ps1 with explicit artifact directories in CI."
}
if (-not (Test-Path -LiteralPath $AppDir)) {
    throw "Local ace-app checkout not found at '$AppDir'. Use package.ps1 with explicit artifact directories in CI."
}

if (-not $Version) {
    $cargoToml = Join-Path $EngineDir "Cargo.toml"
    $match = Select-String -Path $cargoToml -Pattern '^version\s*=\s*"(.+)"'
    if ($match) {
        $Version = $match.Matches.Groups[1].Value
    } else {
        throw "Cannot determine version from ace-engine/Cargo.toml."
    }
}

Write-Host "================================================================" -ForegroundColor Green
Write-Host "  Ace Blocker local release build - v$Version" -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Green

Write-Host "`n[1/2] Building ace-engine..." -ForegroundColor Cyan
Push-Location $EngineDir
try {
    cargo build --release -p ace-engine -p ace-watchdog --bins
    if ($LASTEXITCODE -ne 0) {
        throw "ace-engine build failed."
    }
} finally {
    Pop-Location
}

Write-Host "`n[2/2] Building ace-app..." -ForegroundColor Cyan
Push-Location $AppDir
try {
    # The product MSI is assembled below by our WiX packager. Avoid
    # redundant Tauri-generated installers and normalize CI for Tauri CLI.
    if ($env:CI -eq '1') { $env:CI = 'true' }
    bun run tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "ace-app build failed."
    }
} finally {
    Pop-Location
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ace-installer-$PID-$([Guid]::NewGuid().ToString('N'))"
$engineInput = Join-Path $tempRoot "engine"
$appInput = Join-Path $tempRoot "app"
New-Item -ItemType Directory -Path $engineInput -Force | Out-Null
New-Item -ItemType Directory -Path $appInput -Force | Out-Null

try {
    $engineTarget = Join-Path $ScriptDir "..\target\release"
    foreach ($name in @("ace-engine.exe", "ace-watchdog.exe", "ace-redirect.exe", "install-cert.exe")) {
        $source = Join-Path $engineTarget $name
        if (-not (Test-Path -LiteralPath $source)) {
            throw "$name not found at '$source'."
        }
        Copy-Item -LiteralPath $source -Destination $engineInput
    }

    $wintunRoot = Join-Path $ScriptDir "..\crates\ace-tun\thirdparty\wintun"
    Copy-Item -LiteralPath (Join-Path $wintunRoot "bin\amd64\wintun.dll") -Destination $engineInput
    Copy-Item -LiteralPath (Join-Path $wintunRoot "LICENSE.txt") -Destination (Join-Path $engineInput "WINTUN-LICENSE.txt")

    $appBinary = Join-Path $ScriptDir "..\target\release\ace-app.exe"
    if (-not (Test-Path -LiteralPath $appBinary)) {
        throw "ace-app.exe not found at '$appBinary'."
    }
    Copy-Item -LiteralPath $appBinary -Destination $appInput

    $packageArgs = @{
        Version = $Version
        EngineArtifacts = $engineInput
        AppArtifacts = $appInput
    }
    if ($OutputDir) {
        $packageArgs.OutputDir = $OutputDir
    }

    & $PackageScript @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Installer packaging failed."
    }
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

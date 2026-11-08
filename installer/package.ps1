#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Packages pre-built Ace Blocker release inputs into the canonical WiX MSI.

.DESCRIPTION
    This script is deliberately source-layout agnostic. It does not build ace-engine
    or ace-app and never reaches into sibling repositories. Callers must provide
    explicit artifact directories containing the release binaries.

.PARAMETER Version
    Three-part MSI version (for example 0.3.0).

.PARAMETER EngineArtifacts
    Directory containing ace-engine.exe, ace-watchdog.exe, ace-redirect.exe,
    install-cert.exe, wintun.dll, and WINTUN-LICENSE.txt.

.PARAMETER AppArtifacts
    Directory containing ace-app.exe.

.PARAMETER BundledPacks
    Directory containing manifest.json and the offline site-pack files. Defaults to
    this repository's bundled-packs directory.

.PARAMETER OutputDir
    Destination directory for the MSI. Defaults to this repository's dist directory.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [Parameter(Mandatory = $true)]
    [string]$EngineArtifacts,

    [Parameter(Mandatory = $true)]
    [string]$AppArtifacts,

    [string]$BundledPacks,

    [string]$OutputDir
)

$ErrorActionPreference = "Stop"

if (-not $IsWindows) {
    throw "Ace Blocker MSI packaging requires Windows."
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Version '$Version' is invalid. WiX/MSI releases require a three-part version such as 0.3.0."
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Artifacts = Join-Path $ScriptDir "artifacts"
if (-not $BundledPacks) {
    $BundledPacks = Join-Path $ScriptDir "bundled-packs"
}
if (-not $OutputDir) {
    $OutputDir = Join-Path $ScriptDir "dist"
}

$EngineArtifacts = (Resolve-Path -LiteralPath $EngineArtifacts).Path
$AppArtifacts = (Resolve-Path -LiteralPath $AppArtifacts).Path
$BundledPacks = (Resolve-Path -LiteralPath $BundledPacks).Path

Write-Host "================================================================" -ForegroundColor Green
Write-Host "  Ace Blocker Installer packaging — v$Version" -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Green

# Prefer the pinned .NET global WiX tool. A machine-wide installation can select
# an incompatible CLI (notably WiX v7) even when v4 is also installed.
$DotnetWix = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
$WixCommand = if (Test-Path -LiteralPath $DotnetWix) {
    $DotnetWix
} else {
    (Get-Command "wix" -ErrorAction SilentlyContinue).Source
}

if (-not $WixCommand) {
    throw "WiX Toolset v4 CLI is required. Install it with: dotnet tool install -g wix --version 4.0.5"
}

$WixVersion = (& $WixCommand --version).Trim()
if ($LASTEXITCODE -ne 0 -or $WixVersion -notmatch '^4\.') {
    throw "WiX Toolset v4 is required, but '$WixCommand' reports '$WixVersion'."
}
Write-Host "  WiX $WixVersion ($WixCommand)"

if (Test-Path -LiteralPath $Artifacts) {
    Remove-Item -LiteralPath $Artifacts -Recurse -Force
}
New-Item -ItemType Directory -Path $Artifacts -Force | Out-Null
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

function Copy-RequiredFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceDirectory,

        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $source = Join-Path $SourceDirectory $Name
    if (-not (Test-Path -LiteralPath $source)) {
        throw "Required release input '$Name' was not found at '$source'."
    }

    Copy-Item -LiteralPath $source -Destination $Artifacts
    Write-Host "  $Name"
}

Write-Host "`n[1/3] Staging explicit release inputs..." -ForegroundColor Cyan
foreach ($name in @(
    "ace-engine.exe",
    "ace-watchdog.exe",
    "ace-redirect.exe",
    "install-cert.exe",
    "wintun.dll",
    "WINTUN-LICENSE.txt"
)) {
    Copy-RequiredFile -SourceDirectory $EngineArtifacts -Name $name
}
Copy-RequiredFile -SourceDirectory $AppArtifacts -Name "ace-app.exe"

# Check all packaged PE files, including third-party native dependencies, before
# creating an MSI. No Visual C++ runtime redist/DLL is bundled as a workaround.
& (Join-Path $ScriptDir "verify-crt.ps1") -Paths @(
    (Join-Path $Artifacts "ace-engine.exe"),
    (Join-Path $Artifacts "ace-watchdog.exe"),
    (Join-Path $Artifacts "ace-redirect.exe"),
    (Join-Path $Artifacts "install-cert.exe"),
    (Join-Path $Artifacts "ace-app.exe"),
    (Join-Path $Artifacts "wintun.dll")
)
if ($LASTEXITCODE -ne 0) {
    throw "CRT dependency verification failed."
}

# The DLL must remain Authenticode-signed by WireGuard LLC. Repacking or
# recompressing it breaks the signature and the kernel component will not load.
$WintunSig = Get-AuthenticodeSignature (Join-Path $Artifacts "wintun.dll")
if ($WintunSig.Status -ne "Valid") {
    throw "wintun.dll signature is '$($WintunSig.Status)', expected 'Valid'."
}
Write-Host "    wintun signature: $($WintunSig.Status) ($($WintunSig.SignerCertificate.Subject -replace ',.*',''))"

$ShutdownScript = Join-Path $ScriptDir "shutdown-engine.ps1"
if (-not (Test-Path -LiteralPath $ShutdownScript)) {
    throw "shutdown-engine.ps1 not found at '$ShutdownScript'."
}
Copy-Item -LiteralPath $ShutdownScript -Destination $Artifacts
Write-Host "  shutdown-engine.ps1"

$VerifierScript = Join-Path $ScriptDir "verify-product-signature.ps1"
if (-not (Test-Path -LiteralPath $VerifierScript)) {
    throw "verify-product-signature.ps1 is required for authenticated MSI staging."
}
Copy-Item -LiteralPath $VerifierScript -Destination $Artifacts
Write-Host "  verify-product-signature.ps1"

$BrowserProxyGuardScript = Join-Path $ScriptDir "browser-proxy-extension-policy.ps1"
if (-not (Test-Path -LiteralPath $BrowserProxyGuardScript)) {
    throw "browser-proxy-extension-policy.ps1 is required for optional managed browser protection."
}
Copy-Item -LiteralPath $BrowserProxyGuardScript -Destination $Artifacts
Write-Host "  browser-proxy-extension-policy.ps1 (opt-in)"

Write-Host "`n[2/3] Staging offline site packs..." -ForegroundColor Cyan
$BundleManifest = Join-Path $BundledPacks "manifest.json"
if (-not (Test-Path -LiteralPath $BundleManifest)) {
    throw "Bundled site-pack manifest not found at '$BundleManifest'. Release MSIs must contain an explicit offline pack set."
}

$BundleStaging = Join-Path $Artifacts "bundled-packs"
Copy-Item -LiteralPath $BundledPacks -Destination $BundleStaging -Recurse

$BundleWxs = Join-Path $ScriptDir "bundled-packs.wxs"
$manifest = Get-Content -LiteralPath $BundleManifest -Raw | ConvertFrom-Json

function New-DeterministicGuid([string]$name) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes("ace-bundled-pack-v1:$name")
    $hash = [System.Security.Cryptography.SHA1]::Create().ComputeHash($bytes)
    $g = New-Object byte[] 16
    [Array]::Copy($hash, 0, $g, 0, 16)
    $g[6] = (($g[6] -band 0x0F) -bor 0x50)
    $g[8] = (($g[8] -band 0x3F) -bor 0x80)
    $hex = ($g | ForEach-Object { $_.ToString("x2") }) -join ""
    return ("{0}-{1}-{2}-{3}-{4}" -f
        $hex.Substring(0, 8), $hex.Substring(8, 4), $hex.Substring(12, 4),
        $hex.Substring(16, 4), $hex.Substring(20, 12)).ToUpperInvariant()
}

function ConvertTo-WixIdentifier([string]$value) {
    return ($value -replace '[^A-Za-z0-9_.]', '_')
}

$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine('<?xml version="1.0" encoding="UTF-8"?>')
[void]$sb.AppendLine('<!-- GENERATED by package.ps1 from bundled-packs/manifest.json. Do not edit. -->')
[void]$sb.AppendLine('<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">')
[void]$sb.AppendLine('  <Fragment>')
[void]$sb.AppendLine('    <DirectoryRef Id="BundledPacksDir">')

foreach ($pack in $manifest.packs) {
    $idSafe = ConvertTo-WixIdentifier $pack.id
    $dirId = "BundledDir_$idSafe"
    $compId = "BundledPack_$idSafe"
    $fileId = "F_Bundled_$idSafe"
    $guid = New-DeterministicGuid $pack.id
    $source = "artifacts\bundled-packs\$($pack.file)"
    [void]$sb.AppendLine(('      <Directory Id="{0}" Name="{1}">' -f $dirId, $pack.id))
    [void]$sb.AppendLine(('        <Component Id="{0}" Guid="{{{1}}}">' -f $compId, $guid))
    [void]$sb.AppendLine(('          <File Id="{0}" Name="pack.db" Source="{1}" KeyPath="yes" />' -f $fileId, $source))
    [void]$sb.AppendLine(('          <RemoveFolder Id="RF_{0}" On="uninstall" />' -f $compId))
    [void]$sb.AppendLine('        </Component>')
    [void]$sb.AppendLine('      </Directory>')
}

$manifestGuid = New-DeterministicGuid "manifest.json"
[void]$sb.AppendLine('      <Component Id="BundledManifest" Guid="{' + $manifestGuid + '}">')
[void]$sb.AppendLine('        <File Id="F_BundledManifest" Name="manifest.json" Source="artifacts\bundled-packs\manifest.json" KeyPath="yes" />')
[void]$sb.AppendLine('        <RemoveFolder Id="RF_BundledManifest" On="uninstall" />')
[void]$sb.AppendLine('      </Component>')
[void]$sb.AppendLine('    </DirectoryRef>')
[void]$sb.AppendLine('  </Fragment>')
[void]$sb.AppendLine('  <Fragment>')
[void]$sb.AppendLine('    <ComponentGroup Id="BundledPacksComponents">')
foreach ($pack in $manifest.packs) {
    $compId = "BundledPack_$(ConvertTo-WixIdentifier $pack.id)"
    [void]$sb.AppendLine(('      <ComponentRef Id="{0}" />' -f $compId))
}
[void]$sb.AppendLine('      <ComponentRef Id="BundledManifest" />')
[void]$sb.AppendLine('    </ComponentGroup>')
[void]$sb.AppendLine('  </Fragment>')
[void]$sb.AppendLine('</Wix>')
Set-Content -LiteralPath $BundleWxs -Value $sb.ToString() -Encoding utf8
Write-Host '  generated bundled-packs.wxs'

Write-Host ''
Write-Host '[3/3] Building MSI...' -ForegroundColor Cyan
$InstallerSource = Join-Path $ScriptDir 'installer.wxs'
$GeneratedInstallerSource = Join-Path $ScriptDir 'installer.generated.wxs'
$installerXml = Get-Content -LiteralPath $InstallerSource -Raw
$versionMatches = [regex]::Matches($installerXml, 'Version="\d+\.\d+\.\d+"')
if ($versionMatches.Count -ne 1) {
    throw ('Expected exactly one three-part Version attribute in installer.wxs, found {0}.' -f $versionMatches.Count)
}
$generatedXml = [regex]::Replace(
    $installerXml,
    'Version="\d+\.\d+\.\d+"',
    ('Version="{0}"' -f $Version),
    1
)

try {
    Set-Content -LiteralPath $GeneratedInstallerSource -Value $generatedXml -Encoding utf8
    $outputPath = Join-Path $OutputDir ('AceBlocker-{0}.msi' -f $Version)

    Push-Location $ScriptDir
    try {
        & $WixCommand build 'installer.generated.wxs' 'bundled-packs.wxs' -ext WixToolset.Util.wixext -arch x64 -o $outputPath
        if ($LASTEXITCODE -ne 0) {
            throw 'WiX build failed.'
        }
    } finally {
        Pop-Location
    }
} finally {
    if (Test-Path -LiteralPath $GeneratedInstallerSource) {
        Remove-Item -LiteralPath $GeneratedInstallerSource -Force
    }
}

Write-Host ''
Write-Host '================================================================' -ForegroundColor Green
Write-Host ('  Output: {0}' -f $outputPath) -ForegroundColor Green
Write-Host '================================================================' -ForegroundColor Green

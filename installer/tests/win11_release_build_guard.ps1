# Run only on a disposable Windows packaging VM with WiX 4 installed.
# Uses two genuinely cross-compiled engine artifacts; never installs the
# test-only engine. A normal MSI and Setup EXE are retained in OutputDir.
param(
    [Parameter(Mandatory = $true)] [string] $PackageScript,
    [Parameter(Mandatory = $true)] [string] $EngineArtifacts,
    [Parameter(Mandatory = $true)] [string] $AppArtifacts,
    [Parameter(Mandatory = $true)] [string] $NoLicenseEngine,
    [Parameter(Mandatory = $true)] [string] $Version,
    [Parameter(Mandatory = $true)] [string] $OutputDir
)
$ErrorActionPreference = "Stop"
$PackageScript = (Resolve-Path -LiteralPath $PackageScript).Path
$EngineArtifacts = (Resolve-Path -LiteralPath $EngineArtifacts).Path
$AppArtifacts = (Resolve-Path -LiteralPath $AppArtifacts).Path
$NoLicenseEngine = (Resolve-Path -LiteralPath $NoLicenseEngine).Path
if (Test-Path -LiteralPath $OutputDir) {
    throw "OutputDir must not exist (prevents stale artifact success)."
}
$root = Join-Path ([IO.Path]::GetTempPath()) ("ace-guard-qa-" + [Guid]::NewGuid())
$negativeEngine = Join-Path $root "engine"
$negativeOutput = Join-Path $root "dist"
New-Item -ItemType Directory -Path $negativeEngine -Force | Out-Null
try {
    Copy-Item -LiteralPath (Get-ChildItem -LiteralPath $EngineArtifacts -File).FullName -Destination $negativeEngine
    Copy-Item -LiteralPath $NoLicenseEngine -Destination (Join-Path $negativeEngine "ace-engine.exe") -Force
    $pwsh = Join-Path $PSHOME "pwsh.exe"
    $negativeLog = (& $pwsh -NoProfile -NonInteractive -File $PackageScript -Version $Version -EngineArtifacts $negativeEngine -AppArtifacts $AppArtifacts -OutputDir $negativeOutput 2>&1 | Out-String)
    $negativeExit = $LASTEXITCODE
    Write-Output $negativeLog
    if ($negativeExit -eq 0 -or $negativeLog -notmatch "not a license-enforced release build") {
        throw "Test-only engine was not rejected for the expected reason."
    }
    if (Get-ChildItem -LiteralPath $negativeOutput -File -ErrorAction SilentlyContinue) {
        throw "Rejected package unexpectedly emitted an artifact."
    }
    Write-Output "PASS: actual no-license binary rejected before WiX."
    & $pwsh -NoProfile -NonInteractive -File $PackageScript -Version $Version -EngineArtifacts $EngineArtifacts -AppArtifacts $AppArtifacts -OutputDir $OutputDir
    if ($LASTEXITCODE -ne 0) { throw "Normal release packaging failed." }
    foreach ($file in @("AceBlocker-$Version.msi", "AceBlocker-Setup-$Version.exe")) {
        $artifact = Join-Path $OutputDir $file
        if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
            throw "Expected artifact missing: $file"
        }
        Get-FileHash -LiteralPath $artifact
    }
    Write-Output "PASS: normal license-enforced binary packaged."
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force
}

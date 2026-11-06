#!/usr/bin/env pwsh
<#
Verify the Windows MSI with the operating system's Authenticode trust policy,
plus the pinned SHA-256 of the exact DER publisher certificate.
No network-derived or user-editable MSI property can supply the expected pin.
Invoke only from an approved signing/release environment.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedPublisherCertSha256
)
$ErrorActionPreference = "Stop"

if ($ExpectedPublisherCertSha256 -notmatch '^[a-fA-F0-9]{64}$') {
    throw "Missing or invalid production publisher-certificate SHA-256 pin."
}
$resolved = (Resolve-Path -LiteralPath $MsiPath -ErrorAction Stop).Path
$signature = Get-AuthenticodeSignature -LiteralPath $resolved
if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $signature.SignerCertificate) {
    throw "MSI Authenticode signature is not valid: $($signature.Status)"
}
# Use the DER bytes directly: PowerShell 5.1 can run with .NET Framework
# versions that lack the GetCertHashString(HashAlgorithmName) overload.
$sha256 = [System.Security.Cryptography.SHA256]::Create()
try {
    $actualBytes = $sha256.ComputeHash($signature.SignerCertificate.GetRawCertData())
    $actual = ([System.BitConverter]::ToString($actualBytes)).Replace("-", "")
} finally {
    $sha256.Dispose()
}
if ($actual -ne $ExpectedPublisherCertSha256) {
    throw "MSI Authenticode publisher certificate does not match the approved release pin."
}
$hasCodeSigningEku = $false
foreach ($extension in $signature.SignerCertificate.Extensions) {
    if ($extension -is [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]) {
        foreach ($usage in $extension.EnhancedKeyUsages) {
            if ($usage.Value -eq '1.3.6.1.5.5.7.3.3') {
                $hasCodeSigningEku = $true
            }
        }
    }
}
if (-not $hasCodeSigningEku) {
    throw "MSI signer certificate does not explicitly allow code signing."
}
Write-Host "MSI Authenticode valid; publisher certificate SHA-256 matches approved pin."

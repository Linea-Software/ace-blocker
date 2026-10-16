# shutdown-engine.ps1 - Strict graceful shutdown + watchdog maintenance gate
# for Ace Blocker uninstall/upgrade.
#
# This action runs only AFTER ace-engine.exe --check-locks has succeeded. The
# engine remains the authority for lock semantics: POST /system/shutdown checks
# the network-block and Resistance Mode locks again, then requests a bounded
# AceEngineWatchdog maintenance window before it begins graceful teardown.
#
# Contract: FAIL SAFE. Exit 0 only after an authenticated HTTP 200 response and
# after the AceEngine SCM service reaches Stopped. Any inability to verify that
# sequence exits non-zero so MSI does not stop the watchdog or remove files.
#
# Runs as SYSTEM (deferred, impersonate=no) under Windows PowerShell 5.1.
# Keep this file pure ASCII: PS 5.1 may read a BOM-less script as ANSI.

$ErrorActionPreference = "Stop"

$AppDataDir = Join-Path $env:ProgramData "LineaSoftware\AceBlocker"
$LogPath    = Join-Path $env:TEMP "AceBlocker-uninstall-shutdown.log"
$StopTimeoutSeconds = 30

function Write-Log([string]$message) {
    $line = "{0}  {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $message
    Add-Content -LiteralPath $LogPath -Value $line -ErrorAction SilentlyContinue
}

function Fail-Shutdown([int]$exitCode, [string]$message) {
    Write-Log $message
    Write-Log "=== aborting uninstall/upgrade preparation (exit $exitCode) ==="
    exit $exitCode
}

try {
    Write-Log "=== Ace Blocker engine shutdown + watchdog maintenance ==="

    $portFile  = Join-Path $AppDataDir "port.lock"
    $tokenFile = Join-Path $AppDataDir "capability.token"

    if (-not (Test-Path -LiteralPath $portFile)) {
        Fail-Shutdown 20 "port.lock is missing; engine state cannot be verified safely."
    }
    if (-not (Test-Path -LiteralPath $tokenFile)) {
        Fail-Shutdown 20 "capability.token is missing; shutdown cannot be authenticated."
    }

    $port = ((Get-Content -LiteralPath $portFile -Raw) -replace '\s', '')
    if ($port -notmatch '^\d+$') {
        Fail-Shutdown 20 "port.lock is unparseable; engine state cannot be verified safely."
    }

    # SECURITY: never write this value to the log or command line.
    $token = (Get-Content -LiteralPath $tokenFile -Raw).Trim()
    if (-not $token) {
        Fail-Shutdown 20 "capability.token is empty; shutdown cannot be authenticated."
    }

    $uri = "https://127.0.0.1:$port/system/shutdown"
    Write-Log "Requesting authenticated graceful shutdown on the local engine API."

    # Windows PowerShell 5.1 has no -SkipCertificateCheck. A compiled
    # ICertificatePolicy avoids the runspace-thread problem of a scriptblock
    # ServerCertificateValidationCallback for this loopback-only connection.
    Add-Type -TypeDefinition @"
using System.Net;
using System.Security.Cryptography.X509Certificates;
public sealed class AceTrustAllCertificatePolicy : ICertificatePolicy {
    public bool CheckValidationResult(ServicePoint sp, X509Certificate cert, WebRequest req, int problem) { return true; }
}
"@
    [System.Net.ServicePointManager]::CertificatePolicy = [AceTrustAllCertificatePolicy]::new()
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12

    try {
        $null = Invoke-RestMethod -Method Post -Uri $uri `
            -Headers @{ "X-Ace-Capability" = $token } -TimeoutSec 10
        Write-Log "Shutdown accepted (HTTP 200); watchdog maintenance was acknowledged by the engine."
    } catch {
        $statusCode = $null
        if ($_.Exception.Response) {
            $statusCode = [int]$_.Exception.Response.StatusCode
        }

        if ($statusCode -eq 423) {
            Fail-Shutdown 23 "Engine refused shutdown (HTTP 423): a protection lock is active."
        }
        if ($statusCode -eq 401) {
            Fail-Shutdown 24 "Engine rejected the capability token (HTTP 401)."
        }
        if ($statusCode) {
            Fail-Shutdown 24 "Engine shutdown request failed with HTTP $statusCode."
        }
        Fail-Shutdown 24 "Engine shutdown request could not be completed; engine state is not safely verified."
    }

    # Do not stop the watchdog until the engine has completed teardown and has
    # reported SERVICE_STOPPED. This avoids racing Wintun/database teardown or
    # removing an executable that is still in use.
    $deadline = (Get-Date).AddSeconds($StopTimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $service = Get-Service -Name "AceEngine" -ErrorAction SilentlyContinue
        if (-not $service) {
            Fail-Shutdown 25 "AceEngine service registration disappeared before graceful stop could be verified."
        }
        if ($service.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
            Write-Log "AceEngine reached SERVICE_STOPPED; installer may stop the watchdog."
            Write-Log "=== maintenance preparation complete (exit 0) ==="
            exit 0
        }
        Start-Sleep -Milliseconds 250
    }

    Fail-Shutdown 25 "Timed out waiting for AceEngine to reach SERVICE_STOPPED."
} catch {
    Write-Log "Unexpected shutdown-preparation error: $($_.Exception.Message)"
    Write-Log "=== aborting uninstall/upgrade preparation (exit 26) ==="
    exit 26
}

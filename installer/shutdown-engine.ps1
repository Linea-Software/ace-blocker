# shutdown-engine.ps1 - Strict graceful shutdown + watchdog maintenance gate
# for Ace Blocker uninstall/upgrade.
#
# This action runs only AFTER ace-engine.exe --check-locks has succeeded. The
# engine remains the authority for lock semantics: the typed shutdown RPC checks
# the network-block and Resistance Mode locks again, then requests a bounded
# AceEngineWatchdog maintenance window before it begins graceful teardown.
#
# Contract: FAIL SAFE. Exit 0 only after an authenticated RPC acknowledgement and
# after both SCM services reach Stopped. Any inability to verify that sequence
# exits non-zero so MSI cannot remove files. The watchdog itself still enforces
# the authenticated, bounded maintenance lease.
#
# Runs as SYSTEM (deferred, impersonate=no) under Windows PowerShell 5.1.
# Keep this file pure ASCII: PS 5.1 may read a BOM-less script as ANSI.

$ErrorActionPreference = "Stop"

$AppDataDir = Join-Path $env:ProgramData "LineaSoftware\AceBlocker"
$LogPath = Join-Path $env:TEMP "AceBlocker-uninstall-shutdown.log"
$StopTimeoutSeconds = 30

function Write-Log ([string] $message) {
    $line = "{0}  {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $message
    Add-Content -LiteralPath $LogPath -Value $line -ErrorAction SilentlyContinue
}

function Fail-Shutdown ( [int] $exitCode, [string] $message ) {
    Write-Log $message
    Write-Log "=== aborting uninstall/upgrade preparation (exit $exitCode) ==="
    exit $exitCode
}

try {
    Write-Log "=== Ace Blocker engine shutdown + watchdog maintenance ==="

    $tokenFile = Join-Path $AppDataDir "capability.token"
    $engineExe = Join-Path $PSScriptRoot "ace-engine.exe"

    if (-not (Test-Path -LiteralPath $tokenFile)) {
        Fail-Shutdown 20 "Capability token missing; cannot authenticate maintenance request."
    }
    if (-not (Test-Path -LiteralPath $engineExe)) {
        Fail-Shutdown 20 "Engine maintenance client executable missing."
    }

    # The token never enters PowerShell variables/command lines: the Rust
    # binary reads it privately. LocalSystem service checks locks again and
    # obtains its authenticated watchdog maintenance lease before returning 0.
    Write-Log "Requesting typed Remoc shutdown via engine named pipe."
    $process = Start-Process -FilePath $engineExe -ArgumentList "--request-shutdown" -Wait -PassThru
    $resultCode = $process.ExitCode
    if ($resultCode -eq 23) {
        Fail-Shutdown 23 "Engine refused shutdown: an enforcement lock is active."
    }
    if ($resultCode -ne 0) {
        Fail-Shutdown 24 "Authenticated engine shutdown request failed (exit $resultCode)."
    }
    Write-Log "Engine accepted shutdown and watchdog maintenance."

    # Do not stop the watchdog until the engine has completed teardown and has
    # reported SERVICE_STOPPED. This avoids racing Wintun/database teardown or
    # removing an executable that is still in use.
    $deadline = (Get-Date).AddSeconds($StopTimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $service = Get-Service -Name "AceEngine" -ErrorAction SilentlyContinue
        if (-not $service) {
            Fail-Shutdown 25 "AceEngine service registration disappeared before graceful stop could be verified."
        }
        if (
            $service.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Stopped
        ) {
            Write-Log "AceEngine reached SERVICE_STOPPED; requesting watchdog stop."
            break
        }
        Start-Sleep -Milliseconds 250
    }

    if ((Get-Service -Name "AceEngine").Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
        Fail-Shutdown 25 "Timed out waiting for AceEngine to reach SERVICE_STOPPED."
    }

    # SCM can transiently return ERROR_SERVICE_CANNOT_ACCEPT_CTRL (1061).
    # Do not ignore it and let MSI enter destructive StopServices actions.
    # Recheck status and retry only within the already-authorized maintenance
    # lease; a missing service or a rejected stop remains a failure.
    $deadline = (Get-Date).AddSeconds($StopTimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $watchdog = Get-Service -Name "AceEngineWatchdog"
        if ($watchdog.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
            Write-Log "Both services reached SERVICE_STOPPED."
            Write-Log "=== maintenance preparation complete (exit 0) ==="
            exit 0
        }
        if ($watchdog.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Running) {
            $stop = Start-Process -FilePath "$env:SystemRoot\System32\sc.exe" -ArgumentList "stop AceEngineWatchdog" -Wait -PassThru -WindowStyle Hidden
            $code = $stop.ExitCode
            if ($code -notin @(0, 1061, 1062)) {
                Fail-Shutdown 27 "Watchdog stop rejected by SCM (exit $code)."
            }
            Write-Log "Watchdog SCM stop exit $code; verifying state."
        }
        Start-Sleep -Milliseconds 500
    }
    Fail-Shutdown 28 "Timed out waiting for AceEngineWatchdog to reach SERVICE_STOPPED."
} catch {
    Write-Log "Unexpected shutdown-preparation error: $($_.Exception.Message)"
    Write-Log "=== aborting uninstall/upgrade preparation (exit 26) ==="
    exit 26
}

# verify-watchdog.ps1 - Windows VM validation helper for the Ace userspace watchdog.
# Run from an elevated PowerShell after installing a freshly built MSI.

[CmdletBinding()]
param(
    [switch]$ExerciseRecovery,
    [int]$RecoveryTimeoutSeconds = 45
)

$ErrorActionPreference = "Stop"

function Write-Section([string]$title) {
    Write-Host ""
    Write-Host "=== $title ===" -ForegroundColor Cyan
}

function Invoke-Sc([string[]]$arguments) {
    & "$env:SystemRoot\System32\sc.exe" @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "sc.exe $($arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Get-ServiceProcessId([string]$serviceName) {
    $service = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
    if (-not $service) {
        throw "Service '$serviceName' is not installed."
    }
    return [uint32]$service.ProcessId
}

function Wait-ForRunningServiceWithNewPid(
    [string]$serviceName,
    [uint32]$oldProcessId,
    [int]$timeoutSeconds
) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $service = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
        if ($service -and $service.State -eq "Running" -and
            [uint32]$service.ProcessId -ne 0 -and
            [uint32]$service.ProcessId -ne $oldProcessId) {
            return [uint32]$service.ProcessId
        }
        Start-Sleep -Milliseconds 500
    }
    throw "Timed out waiting for '$serviceName' to recover with a new PID."
}

function Exercise-ServiceKillRecovery([string]$serviceName) {
    $oldProcessId = Get-ServiceProcessId $serviceName
    if ($oldProcessId -eq 0) {
        throw "Service '$serviceName' has no running process."
    }

    Write-Host "Killing $serviceName PID $oldProcessId once..."
    Stop-Process -Id $oldProcessId -Force
    $newProcessId = Wait-ForRunningServiceWithNewPid `
        $serviceName $oldProcessId $RecoveryTimeoutSeconds
    Write-Host "PASS: $serviceName recovered as PID $newProcessId" -ForegroundColor Green
}

Write-Section "Service configuration"
foreach ($serviceName in @("AceEngine", "AceEngineWatchdog")) {
    Write-Host "-- $serviceName --" -ForegroundColor Yellow
    Invoke-Sc @("qc", $serviceName)
    Invoke-Sc @("qfailure", $serviceName)
    Invoke-Sc @("qfailureflag", $serviceName)
    Invoke-Sc @("qsidtype", $serviceName)
    Invoke-Sc @("sdshow", $serviceName)
    Get-CimInstance Win32_Service -Filter "Name='$serviceName'" |
        Select-Object Name, State, StartMode, PathName, ProcessId |
        Format-List
}

$installDir = Join-Path $env:ProgramFiles "Ace Blocker"
$programDataDir = Join-Path $env:ProgramData "LineaSoftware\AceBlocker"

Write-Section "Installed file ACLs"
foreach ($path in @(
    (Join-Path $installDir "ace-engine.exe"),
    (Join-Path $installDir "ace-watchdog.exe"),
    (Join-Path $installDir "wintun.dll")
)) {
    if (Test-Path -LiteralPath $path) {
        & "$env:SystemRoot\System32\icacls.exe" $path
    } else {
        Write-Warning "Missing expected installed file: $path"
    }
}

Write-Section "ProgramData ACL"
if (Test-Path -LiteralPath $programDataDir) {
    & "$env:SystemRoot\System32\icacls.exe" $programDataDir
} else {
    Write-Warning "ProgramData directory not present yet: $programDataDir"
}

if ($ExerciseRecovery) {
    Write-Section "Single-process recovery"
    Exercise-ServiceKillRecovery "AceEngine"
    Start-Sleep -Seconds 3
    Exercise-ServiceKillRecovery "AceEngineWatchdog"
}

Write-Section "Manual validation still required"
@"
1. Reboot: both services return Running and heartbeat remains healthy.
2. Active network-block lock: POST /system/shutdown returns 423; maintenance is
   not entered; killing AceEngine causes recovery; MSI uninstall is blocked.
3. Repeat item 2 with a Resistance Mode lock.
4. Unlocked shutdown: POST /system/shutdown returns 200; AceEngine reaches
   Stopped and remains down during the bounded maintenance window.
5. Debug hang test only: with debug service binaries installed, create
   $programDataDir\watchdog-heartbeat.pause. Confirm one/two missed heartbeats
   do not restart the service, the threshold does, only the current SCM PID is
   terminated, and the service returns healthy. Remove the file afterward.
6. Upgrade: old MSI passes lock gate, opens maintenance, stops engine/watchdog,
   replaces files, and the new MSI starts both services.
7. Normal-user tamper checks: replacing ace-engine.exe/ace-watchdog.exe/wintun.dll
   or modifying protected ProgramData state should be denied.
8. Normal-user and elevated checks: try sc stop, Services UI stop, service path/
   start-type changes, and file replacement. Record which elevated-admin actions
   remain possible; this design is not administrator-proof.
9. Inspect $programDataDir\logs\ace-watchdog.log for recovery/heartbeat/
   maintenance events and confirm it contains no application secrets.
"@ | Write-Host

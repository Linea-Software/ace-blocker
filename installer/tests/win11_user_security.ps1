# Run inside an installed, disposable Windows VM as a non-elevated user.
# This checks access without reading secrets or writing protected files.
param(
    [string]$Probe = 'C:\AceQA\vm_network_probe.exe'
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -or $identity.IsSystem) {
    throw 'Run the QA checks in a non-elevated user session'
}
$root = Join-Path $env:ProgramData 'LineaSoftware\AceBlocker'
$install = Join-Path $env:ProgramFiles 'Ace Blocker'
$failures = 0

function Test-DeniedFileAccess([string]$Path, [IO.FileAccess]$Access) {
    try {
        $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, $Access, [IO.FileShare]::ReadWrite)
        $stream.Dispose()
        Write-Output "FAIL: $Access allowed on $Path"
        $script:failures++
    } catch {
        $cause = $_.Exception
        while ($cause.InnerException) { $cause = $cause.InnerException }
        if ($cause -is [UnauthorizedAccessException]) {
            Write-Output "PASS: $Access denied on $Path"
        } else {
            Write-Output "FAIL: $Access on $Path was not an ACL denial ($($cause.GetType().Name))"
            $script:failures++
        }
    }
}

foreach ($path in @(
    (Join-Path $root 'engine_db.sql'),
    (Join-Path $root 'certs\mitm_ca.key')
)) {
    Test-DeniedFileAccess $path Read
    Test-DeniedFileAccess $path Write
}
foreach ($path in @(
    (Join-Path $install 'ace-engine.exe'),
    (Join-Path $install 'ace-watchdog.exe'),
    (Join-Path $install 'wintun.dll'),
    (Join-Path $root 'capability.token')
)) {
    Test-DeniedFileAccess $path Write
}
& $Probe status
if ($LASTEXITCODE -ne 0) {
    Write-Output 'FAIL: ordinary user could not connect through the normal RPC client'
    $failures++
} else {
    Write-Output 'PASS: ordinary user RPC connection'
}

# Request the STOP access right without actually stopping either service.
# The native handle check distinguishes ACL denial from a non-stoppable service.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AceQaScm {
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr OpenSCManager(IntPtr machine, IntPtr database, uint access);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr OpenService(IntPtr manager, string name, uint access);
    [DllImport("advapi32.dll", SetLastError=true)]
    public static extern bool CloseServiceHandle(IntPtr handle);
}
'@
$scm = [AceQaScm]::OpenSCManager([IntPtr]::Zero, [IntPtr]::Zero, 1)
if ($scm -eq [IntPtr]::Zero) { throw "Cannot connect to SCM: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
try {
    foreach ($service in @('AceEngine', 'AceEngineWatchdog')) {
        foreach ($right in @(2, 32, 65536)) { # CHANGE_CONFIG, STOP, DELETE
            $handle = [AceQaScm]::OpenService($scm, $service, $right)
            $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            if ($handle -ne [IntPtr]::Zero) {
                [void][AceQaScm]::CloseServiceHandle($handle)
                Write-Output "FAIL: $service granted SCM right $right"
                $failures++
            } elseif ($errorCode -eq 5) {
                Write-Output "PASS: $service denied SCM right $right"
            } else {
                Write-Output "FAIL: $service right $right returned unexpected error $errorCode"
                $failures++
            }
        }
    }
} finally { [void][AceQaScm]::CloseServiceHandle($scm) }

Write-Output "User-security failures: $failures"
if ($failures -ne 0) { exit 1 }
exit 0

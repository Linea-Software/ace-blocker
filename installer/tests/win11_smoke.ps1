# Windows 11 runtime smoke suite for a disposable Ace Blocker test VM.
# Run through windows_vm_harness.py as LocalSystem (QEMU guest agent).
# No lock, proxy, service, or registry configuration is modified.
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$checks = New-Object 'System.Collections.Generic.List[object]'

function Add-Check {
    param(
        [string] $Name,
        [bool] $Pass,
        [string] $Detail,
        [string] $Kind = "required"
    )

    $checks.Add(
        [pscustomobject] @{
            name = $Name
            pass = $Pass
            detail = $Detail
            kind = $Kind
        }
    )
}

$os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
Add-Check "Windows 11"([bool]($os -and $os.BuildNumber -ge 22000)) (
    "build=" + $os.BuildNumber
)

$svc = @(
    Get-CimInstance Win32_Service -ErrorAction SilentlyContinue | Where-Object {
        $_.Name -eq "AceEngine" -or $_.Name -eq "AceEngineWatchdog"
    }
)
$exeDir = $null
foreach ($name in @( "AceEngine", "AceEngineWatchdog" )) {
    $item = $svc | Where-Object Name -eq $name | Select-Object -First 1
    Add-Check "$name installed"([bool] $item) (
        $(
            if ($item) {
                $item.PathName
            } else {
                "not found"
            }
        )
    )
    if ($item) {
        Add-Check "$name running"($item.State -eq "Running") $item.State
        Add-Check "$name autostart"($item.StartMode -eq "Auto") $item.StartMode
        Add-Check "$name LocalSystem"(
            $item.StartName -eq "LocalSystem" -or $item.StartName -eq "NT AUTHORITY\SYSTEM"
        ) $item.StartName
        $failure = (sc.exe qfailure $name | Out-String)
        Add-Check "$name SCM recovery"($failure -match "RESTART") (
            "restart=" + [bool]($failure -match "RESTART")
        )
        $pathname = $item.PathName.Trim()
        if ($pathname -match '^\s*"([^"]+\.exe)"') {
            $exe = $Matches[1]
        } elseif ($pathname -match '^\s*(.+?\.exe)(\s|$)') {
            $exe = $Matches[1]
        } else {
            $exe = $pathname
        }
        Add-Check "$name executable"(
            Test-Path -LiteralPath $exe -PathType Leaf
        ) $exe
        if ($name -eq "AceEngine") {
            $exeDir = Split-Path -Parent $exe
        }
    }
}

if ($exeDir) {
    foreach (
        $file in @(
            "ace-engine.exe",
            "ace-watchdog.exe",
            "ace-app.exe",
            "ace-redirect.exe",
            "install-cert.exe",
            "wintun.dll"
        )
    ) {
        $target = Join-Path $exeDir $file
        Add-Check "MSI payload $file"(
            Test-Path -LiteralPath $target -PathType Leaf
        ) $target
    }
    $packs = Join-Path $exeDir "bundled-packs"
    Add-Check "offline packs bundled"(
        Test-Path -LiteralPath $packs -PathType Container
    ) $packs
}

$programs = @(
    Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue |
    Where-Object {
        $_.DisplayName -match "Ace\s*Blocker"
    }
)
$msiInstalled = $programs.Count -gt 0
Add-Check "Windows uninstall registration" $msiInstalled(
    $(
        if ($msiInstalled) {
            (
                $programs | Select-Object -First 1 -ExpandProperty DisplayName
            ) + " " +
            ($programs | Select-Object -First 1 -ExpandProperty DisplayVersion)
        } else {
            "missing"
        }
    )
)

$dataRoot = Join-Path $env:ProgramData "LineaSoftware\AceBlocker"
Add-Check "ProgramData created"(
    Test-Path -LiteralPath $dataRoot -PathType Container
) $dataRoot
$adapters = @(
    Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object {
        $_.Name -eq "Ace Blocker"
    }
)
Add-Check "Ace Blocker network adapter"($adapters.Count -gt 0) (
    $(
        if ($adapters.Count) {
            $adapters[0].Status
        } else {
            "absent"
        }
    )
)

$desktopSessions = @(Get-Process -Name "explorer" -ErrorAction SilentlyContinue)
Add-Check "interactive desktop present"($desktopSessions.Count -gt 0) (
    "explorer processes=" + $desktopSessions.Count
) "informational"
$gui = @(Get-Process -Name "ace-app" -ErrorAction SilentlyContinue)
Add-Check "GUI currently running"($gui.Count -gt 0) (
    "processes=" + $gui.Count
) "informational"

$required = @(
    $checks | Where-Object {
        $_.kind -eq "required"
    }
)
$failed = @(
    $required | Where-Object {
        -not $_.pass
    }
)
[pscustomobject] @{
    timestamp = (Get-Date).ToString("o")
    total = $required.Count
    passed = $required.Count - $failed.Count
    failed = $failed.Count
    checks = $checks.ToArray()
}
    | ConvertTo-Json -Depth 6 -Compress
if ($failed.Count -gt 0) {
    exit 1
}

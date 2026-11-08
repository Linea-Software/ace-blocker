#requires -Version 5.1
<#
.SYNOPSIS
  Opt-in, reversible browser proxy guard for Ace Blocker on Windows.
.DESCRIPTION
  Enforces direct browser connections and blocks extensions requesting the
  proxy permission on Chrome, Edge and supported Firefox versions. Never run
  automatically by Ace or the MSI: machine-wide browser policies have material
  compatibility consequences and require explicit administrator consent.

  Refuses to overwrite existing enterprise/user policies. Saves a narrow
  ownership journal in the protected Ace ProgramData directory before changes.
  Disable removes only values that still match this script's exact payload.
  Firefox blocked_permissions requires Firefox 153+; Firefox Proxy policy is
  available on older releases and disables proxy configuration independently.
.EXAMPLE
  .\browser-proxy-extension-policy.ps1 -Action Status
.EXAMPLE
  .\browser-proxy-extension-policy.ps1 -Action Enable -IUnderstand
.EXAMPLE
  .\browser-proxy-extension-policy.ps1 -Action Disable
#>
[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [ValidateSet('Status', 'Enable', 'Disable', 'DisableIfOwned')]
    [string] $Action = 'Status',
    [switch] $IUnderstand
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Browser proxy protection supports Windows only.'
}
if (-not [Environment]::Is64BitProcess) {
    throw 'Use elevated 64-bit Windows PowerShell to avoid registry redirection.'
}

$policy = '{"*":{"blocked_permissions":["proxy"]}}'
$stateDirectory = Join-Path $env:ProgramData 'LineaSoftware\AceBlocker'
$statePath = Join-Path $stateDirectory 'browser-proxy-extension-policy-state.json'
$extensionRoots = [ordered]@{
    Chrome  = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Policies\Google\Chrome'
    Edge    = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Policies\Microsoft\Edge'
    Firefox = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Policies\Mozilla\Firefox'
}
$firefoxProxyRoot = $extensionRoots.Firefox + '\Proxy'
$oldExtensionPolicies = @(
    'ExtensionSettings', 'ExtensionInstallBlocklist', 'ExtensionInstallAllowlist',
    'ExtensionInstallForcelist', 'ExtensionInstallSources', 'ExtensionAllowedTypes'
)
$oldProxyPolicies = @('ProxyMode', 'ProxyServer', 'ProxyPacUrl', 'ProxyBypassList', 'ProxySettings')

# Each entry is a policy value Ace creates. Ownership is strictly exact-match.
$ownedValues = @(
    [pscustomobject]@{ Label='Chrome extension'; Path=$extensionRoots.Chrome; Name='ExtensionSettings'; Kind='String'; Expected=$policy },
    [pscustomobject]@{ Label='Chrome proxy'; Path=$extensionRoots.Chrome; Name='ProxyMode'; Kind='String'; Expected='direct' },
    [pscustomobject]@{ Label='Edge extension'; Path=$extensionRoots.Edge; Name='ExtensionSettings'; Kind='String'; Expected=$policy },
    [pscustomobject]@{ Label='Edge proxy'; Path=$extensionRoots.Edge; Name='ProxyMode'; Kind='String'; Expected='direct' },
    [pscustomobject]@{ Label='Firefox extension'; Path=$extensionRoots.Firefox; Name='ExtensionSettings'; Kind='MultiString'; Expected=@($policy) },
    [pscustomobject]@{ Label='Firefox proxy mode'; Path=$firefoxProxyRoot; Name='Mode'; Kind='String'; Expected='none' },
    [pscustomobject]@{ Label='Firefox proxy locked'; Path=$firefoxProxyRoot; Name='Locked'; Kind='DWord'; Expected=1 }
)

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Run elevated 64-bit Windows PowerShell.'
    }
}

function Get-ValueRecord($entry) {
    if (-not (Test-Path -LiteralPath $entry.Path)) { return $null }
    $key = Get-Item -LiteralPath $entry.Path
    if ($key.GetValueNames() -cnotcontains $entry.Name) { return $null }
    return [pscustomobject]@{
        Kind = [string]$key.GetValueKind($entry.Name)
        Value = $key.GetValue($entry.Name)
    }
}

function Is-Expected($entry, $record) {
    if ($null -eq $record -or $record.Kind -cne $entry.Kind) { return $false }
    if ($entry.Kind -ceq 'MultiString') {
        $actual = @($record.Value)
        $desired = @($entry.Expected)
        return ($actual.Count -eq $desired.Count -and
            (($actual -join [char]10) -ceq ($desired -join [char]10)))
    }
    if ($entry.Kind -ceq 'DWord') {
        return ([int]$record.Value -eq [int]$entry.Expected)
    }
    return ([string]$record.Value -ceq [string]$entry.Expected)
}

function Read-State {
    if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) { return $null }
    try {
        $state = Get-Content -LiteralPath $statePath -Encoding UTF8 -Raw | ConvertFrom-Json
        if ($state.Schema -ne 2 -or $state.Policy -cne $policy -or
            $state.Browsers -cne 'Chrome,Edge,Firefox') {
            throw 'Unexpected policy owner state format.'
        }
        return $state
    } catch {
        throw "Unrecognized guard state at $statePath. Refusing to overwrite or delete."
    }
}

function Write-State {
    if (-not (Test-Path -LiteralPath $stateDirectory -PathType Container)) {
        throw "Protected Ace ProgramData directory is missing: $stateDirectory. Install Ace first."
    }
    $temporaryPath = Join-Path $stateDirectory ('browser-guard-' + [guid]::NewGuid().ToString('N') + '.tmp')
    try {
        $state = [ordered]@{
            Schema = 2
            Policy = $policy
            Browsers = 'Chrome,Edge,Firefox'
            CreatedUtc = [datetime]::UtcNow.ToString('o')
        }
        $json = ConvertTo-Json -InputObject $state -Compress
        [IO.File]::WriteAllText($temporaryPath, $json, (New-Object System.Text.UTF8Encoding($false)))
        [IO.File]::Move($temporaryPath, $statePath)
    } finally {
        if (Test-Path -LiteralPath $temporaryPath) {
            Remove-Item -LiteralPath $temporaryPath -Force
        }
    }
}

function Assert-NoConflicts {
    # Policy values may also exist under logged-in users' HKCU hives. Refuse
    # conflicts even if HKLM takes precedence in the current browser version.
    $policyRoots = New-Object 'System.Collections.Generic.List[string]'
    foreach ($root in $extensionRoots.Values) { $policyRoots.Add($root) }
    foreach ($hive in @(Get-ChildItem -LiteralPath 'Registry::HKEY_USERS')) {
        $sid = Split-Path -Path $hive.Name -Leaf
        if ($sid -notmatch '^S-1-\d+(?:-\d+)+$') { continue }
        foreach ($vendor in @('Google\Chrome', 'Microsoft\Edge', 'Mozilla\Firefox')) {
            $policyRoots.Add(('Registry::HKEY_USERS\' + $sid + '\SOFTWARE\Policies\' + $vendor))
        }
    }
    foreach ($root in $policyRoots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        $key = Get-Item -LiteralPath $root
        $isFirefox = $root.EndsWith('\Mozilla\Firefox', [StringComparison]::OrdinalIgnoreCase)
        $names = @($oldExtensionPolicies)
        if (-not $isFirefox) { $names += $oldProxyPolicies }
        if ($isFirefox -and ($key.GetSubKeyNames() -contains 'Proxy')) {
            throw "Existing Firefox Proxy enterprise policy at $root. No changes made."
        }
        foreach ($name in $names) {
            if (($key.GetValueNames() -contains $name) -or
                ($key.GetSubKeyNames() -contains $name)) {
                throw "Existing browser policy $name at $root. No changes made."
            }
        }
    }
    # Don't take over a pre-existing Firefox proxy configuration.
    if (Test-Path -LiteralPath $firefoxProxyRoot) {
        throw "Existing Firefox Proxy policy at $firefoxProxyRoot. No changes made."
    }
}

function Assert-NoUnownedChanges {
    # A missing value is allowed: Enable may have been interrupted after
    # journaling and before all registry writes. Different values are never
    # deleted or overwritten.
    foreach ($entry in $ownedValues) {
        $record = Get-ValueRecord $entry
        if ($null -ne $record -and -not (Is-Expected $entry $record)) {
            throw "Conflicting value at $($entry.Path)\$($entry.Name). No changes made."
        }
    }
}

function Show-Status {
    $state = Read-State
    $complete = $null -ne $state
    foreach ($entry in $ownedValues) {
        $record = Get-ValueRecord $entry
        $status = 'absent'
        if ($null -ne $record) {
            $status = if (Is-Expected $entry $record) { 'configured' } else { 'conflict' }
        }
        if ($status -cne 'configured') { $complete = $false }
        Write-Output ($entry.Label + ': ' + $status)
    }
    Write-Output ('ACE-owned state: ' + $(if ($null -eq $state) { 'absent' } else { 'present' }))
    Write-Output ('Policy registry values complete: ' + $(if ($complete) { 'yes' } else { 'no' }))
    Write-Output 'A browser restart/policy refresh is required; inspect chrome://policy, edge://policy and about:policies.'
    Write-Output 'Firefox blocked_permissions requires Firefox 153+; verify compatibility in about:policies.'
}

if ($Action -ceq 'Status') {
    Show-Status
    return
}

Assert-Administrator
$mutex = New-Object System.Threading.Mutex($false, 'Global\AceBlockerBrowserProxyGuard')
$lockHeld = $false
try {
    $lockHeld = $mutex.WaitOne([timespan]::FromSeconds(15))
    if (-not $lockHeld) { throw 'Timed out waiting for another browser-policy operation.' }

    if ($Action -ceq 'DisableIfOwned') { $ConfirmPreference = 'None' }
    if ($Action -ceq 'DisableIfOwned' -and $null -eq (Read-State)) {
        Write-Output 'No Ace-managed browser proxy policies to disable.'
        return
    }
    if ($Action -ceq 'Enable') {
        if (-not $IUnderstand) {
            throw 'Enable requires -IUnderstand. It disables browser proxies, including corporate proxies.'
        }
        if ($null -ne (Read-State)) { throw 'Owned state exists. Use Status/Disable, including after interrupted setup.' }
        Assert-NoConflicts
        if ($PSCmdlet.ShouldProcess('Chrome, Edge, Firefox machine policies', 'Disable browser proxies and proxy API')) {
            # Journal before writes allows reversible partial installation.
            Write-State
            foreach ($entry in $ownedValues) {
                if (-not (Test-Path -LiteralPath $entry.Path)) {
                    New-Item -Path $entry.Path -Force | Out-Null
                }
                New-ItemProperty -Path $entry.Path -Name $entry.Name -PropertyType $entry.Kind -Value $entry.Expected | Out-Null
            }
            Assert-NoUnownedChanges
            Write-Output 'Enabled. Restart browsers and verify the managed policy pages.'
        }
    } elseif ($Action -ceq 'Disable' -or $Action -ceq 'DisableIfOwned') {
        if ($null -eq (Read-State)) {
            throw 'No Ace-owned policy journal; refusing to delete other policies.'
        }
        Assert-NoUnownedChanges
        if ($PSCmdlet.ShouldProcess('Ace-owned browser policy values', 'Remove only exact matching values')) {
            foreach ($entry in $ownedValues) {
                $record = Get-ValueRecord $entry
                if ($null -ne $record) {
                    Remove-ItemProperty -LiteralPath $entry.Path -Name $entry.Name
                }
            }
            Remove-Item -LiteralPath $statePath
            Write-Output 'Owned guard values removed. Other registry values and parent keys are untouched.'
        }
    }
} finally {
    if ($lockHeld) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}

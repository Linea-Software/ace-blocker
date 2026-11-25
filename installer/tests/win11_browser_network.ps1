# Native Edge smoke for a disposable Windows QA VM.
# Run as the interactive standard test user, with the QA example.org block active.
param(
    [string] $OutputPath = "$env:TEMP\ace-browser-qa.json",
    [string] $CaseUrl = ''
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$edge = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft\Edge\Application\msedge.exe'
if (!(Test-Path $edge)) { throw 'Edge not installed' }
$profile = Join-Path $env:TEMP ("ace-edge-qa-" + [Guid]::NewGuid())
$process = $null
$socket = $null
$results = @()
$id = 0
function Send-Cdp([string] $method, [hashtable] $params = @{}) {
    $script:id++
    $message = @{ id = $script:id; method = $method; params = $params } | ConvertTo-Json -Compress -Depth 8
    $bytes = [Text.Encoding]::UTF8.GetBytes($message)
    $deadline = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(5))
    try {
        $socket.SendAsync([ArraySegment[byte]]::new($bytes), [Net.WebSockets.WebSocketMessageType]::Text, $true, $deadline.Token).GetAwaiter().GetResult()
    } finally { $deadline.Dispose() }
}
function Receive-Cdp([Threading.CancellationToken] $token) {
    $buf = New-Object byte[] 65536
    $stream = [IO.MemoryStream]::new()
    try {
        do {
            $frame = $socket.ReceiveAsync([ArraySegment[byte]]::new($buf), $token).GetAwaiter().GetResult()
            if ($frame.MessageType -eq [Net.WebSockets.WebSocketMessageType]::Close) { throw 'CDP closed' }
            $stream.Write($buf, 0, $frame.Count)
            if ($stream.Length -gt 1048576) { throw 'CDP frame too large' }
        } until ($frame.EndOfMessage)
        return ([Text.Encoding]::UTF8.GetString($stream.ToArray()) | ConvertFrom-Json)
    } finally { $stream.Dispose() }
}
function Write-Report([string] $Fatal = '') {
    $report = [pscustomobject]@{ account=[Environment]::UserName; browser=(Get-Item $edge).VersionInfo.ProductVersion; tests=$script:results; fatal=$Fatal }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    $report | ConvertTo-Json -Depth 8
}
try {
    # Disable HTTP-to-HTTPS automatic upgrades in this disposable profile only.
    # Do not change certificate verification, proxy settings or ACE protections.
    $process = Start-Process -FilePath $edge -ArgumentList @(
        '--headless=new','--no-first-run','--no-default-browser-check',
        '--disable-background-networking','--disable-component-update',
        '--disable-features=HttpsUpgrades','--remote-debugging-port=0',
        ('--user-data-dir="{0}"' -f $profile),'about:blank'
    ) -PassThru
    $portFile = Join-Path $profile 'DevToolsActivePort'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (!(Test-Path $portFile)) {
        if ($process.HasExited -or $watch.Elapsed.TotalSeconds -gt 30) { throw 'Edge debugging endpoint unavailable' }
        Start-Sleep -Milliseconds 250
    }
    $port = [int](Get-Content $portFile | Select-Object -First 1)
    $targets = Invoke-RestMethod "http://127.0.0.1:$port/json/list" -TimeoutSec 5
    $page = @($targets | Where-Object type -eq 'page')[0]
    if (!$page.webSocketDebuggerUrl) { throw 'No browser page target' }
    $socket = [Net.WebSockets.ClientWebSocket]::new()
    $connect = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(5))
    try { $socket.ConnectAsync([Uri]$page.webSocketDebuggerUrl,$connect.Token).GetAwaiter().GetResult() } finally { $connect.Dispose() }
    Send-Cdp 'Network.enable'
    Send-Cdp 'Page.enable'
    Send-Cdp 'Network.setCacheDisabled' @{ cacheDisabled = $true }
    $cases = @(
        @{ url='http://example.org/'; expected=404 },
        @{ url='http://example.com/'; expected=200 },
        @{ url='https://example.org/'; expected=404 },
        @{ url='https://example.com/'; expected=200 }
    )
    if ($CaseUrl) { $cases = @($cases | Where-Object { $_.url -eq $CaseUrl }) }
    if (!$cases.Count) { throw 'Unknown smoke-test URL' }
    foreach ($case in $cases) {
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $deadline = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(35))
        $observed = $null
        $failure = $null
        $protocol = $null
        $requestId = $null
        try {
            Send-Cdp 'Page.navigate' @{ url = $case.url }
            while ($null -eq $observed -and !$failure) {
                $event = Receive-Cdp $deadline.Token
                if ($event.method -eq 'Network.requestWillBeSent' -and $event.params.type -eq 'Document' -and $event.params.request.url -eq $case.url) {
                    $requestId = $event.params.requestId
                }
                if ($event.method -eq 'Network.responseReceived' -and $event.params.type -eq 'Document' -and $event.params.response.url -eq $case.url) {
                    $observed = [int]$event.params.response.status
                    $protocol = $event.params.response.protocol
                }
                if ($event.method -eq 'Network.loadingFailed' -and $requestId -and $event.params.requestId -eq $requestId) {
                    $failure = $event.params.errorText
                }
            }
        } catch {
            $failure = 'No exact document response before timeout or CDP error'
        } finally { $deadline.Dispose() }
        $results += [pscustomobject]@{
            url=$case.url; expected=$case.expected; observed=$observed;
            protocol=$protocol; failure=$failure; durationMs=$timer.ElapsedMilliseconds;
            pass=($null -ne $observed -and $observed -eq $case.expected)
        }
        Write-Report
        # A cancelled WebSocket receive aborts that session. Preserve the
        # failed result and let a fresh process run the remaining cases.
        if ($socket.State -ne [Net.WebSockets.WebSocketState]::Open) { break }
    }
    if ($results.Count -ne $cases.Count -or @($results | Where-Object { !$_.pass }).Count) { exit 1 }
    exit 0
} catch {
    Write-Report $_.Exception.Message
    exit 1
} finally {
    if ($socket) {
        try { Send-Cdp 'Browser.close' } catch {}
        $socket.Dispose()
    }
    if ($process -and !$process.HasExited) {
        if (!$process.WaitForExit(3000)) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    }
    Remove-Item -LiteralPath $profile -Recurse -Force -ErrorAction SilentlyContinue
}

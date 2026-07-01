param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$WirelessSerial = "",
    [int]$WirelessPort = 5555,
    [switch]$SkipWirelessAdb,
    [string]$ProjectId = "elon-self",
    [string]$ProjectTitle = "Elon Self Project",
    [string]$ConversationPrefix = "mcp_native_e2e",
    [int]$FirstReplyTimeoutSec = 90,
    [int]$FinishTimeoutSec = 420,
    [int]$PollIntervalSec = 5,
    [switch]$RunGitProbe,
    [switch]$RunPublishProbe,
    [string]$ServerBaseUrl = "http://43.139.149.158:8080",
    [int]$PublishVerifyTimeoutSec = 900,
    [string]$ReportPath = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$InvokeMcpScript = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"

if (!(Test-Path -LiteralPath $Adb)) {
    throw "adb not found: $Adb"
}
if (!(Test-Path -LiteralPath $InvokeMcpScript)) {
    throw "invoke-apk-mcp.ps1 not found: $InvokeMcpScript"
}

function Write-Step {
    param([string]$Message)
    Write-Host "[apk-mcp-e2e] $Message"
}

function ConvertTo-JsonCompact {
    param([object]$Value)
    return ($Value | ConvertTo-Json -Depth 40 -Compress)
}

function Invoke-AdbCommand {
    param(
        [string[]]$AdbArgs,
        [string]$Serial = ""
    )
    $serialArgs = @()
    if ($Serial.Trim()) {
        $serialArgs = @("-s", $Serial.Trim())
    }
    $output = & $Adb @serialArgs @AdbArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        $text = ($output | Out-String).Trim()
        throw "adb $($AdbArgs -join ' ') failed for '$Serial': $text"
    }
    return $output
}

function Get-AdbDevices {
    $lines = & $Adb devices -l 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "adb devices failed: $(($lines | Out-String).Trim())"
    }
    $devices = @()
    foreach ($line in $lines) {
        $text = [string]$line
        if ($text -match "^\s*$" -or $text -match "^List of devices") {
            continue
        }
        if ($text -match "^(\S+)\s+(\S+)(.*)$") {
            $serial = $Matches[1]
            $state = $Matches[2]
            $devices += [pscustomobject]@{
                serial = $serial
                state = $state
                detail = $Matches[3].Trim()
                is_wireless = ($serial -match ":\d+$")
            }
        }
    }
    return $devices
}

function Connect-WirelessDevice {
    param([string]$Target)
    if (!$Target.Trim()) {
        return
    }
    Write-Step "adb connect $Target"
    & $Adb connect $Target | Out-Host
}

function Get-DeviceWifiAddress {
    param([string]$Serial)
    $wlan = Invoke-AdbCommand -Serial $Serial -AdbArgs @("shell", "ip", "-f", "inet", "addr", "show", "wlan0")
    $wlanText = ($wlan | Out-String)
    if ($wlanText -match "\binet\s+(\d+\.\d+\.\d+\.\d+)") {
        return $Matches[1]
    }
    $route = Invoke-AdbCommand -Serial $Serial -AdbArgs @("shell", "ip", "route")
    $routeText = ($route | Out-String)
    if ($routeText -match "\bwlan\d*\b.*\bsrc\s+(\d+\.\d+\.\d+\.\d+)") {
        return $Matches[1]
    }
    if ($routeText -match "\bsrc\s+((?!10\.|127\.|169\.254\.)\d+\.\d+\.\d+\.\d+)") {
        return $Matches[1]
    }
    throw "Could not determine phone Wi-Fi address from adb shell ip output."
}

function Resolve-AdbDevice {
    if ($WirelessSerial.Trim()) {
        Connect-WirelessDevice -Target $WirelessSerial.Trim()
    }

    $devices = @(Get-AdbDevices | Where-Object { $_.state -eq "device" })
    if ($DeviceSerial.Trim()) {
        $wanted = $DeviceSerial.Trim()
        if ($wanted -match ":\d+$" -and !($devices | Where-Object { $_.serial -eq $wanted })) {
            Connect-WirelessDevice -Target $wanted
            $devices = @(Get-AdbDevices | Where-Object { $_.state -eq "device" })
        }
        $match = $devices | Where-Object { $_.serial -eq $wanted } | Select-Object -First 1
        if ($match) {
            return $match
        }
        throw "Requested ADB device is not online: $wanted"
    }

    $usb = $devices | Where-Object { -not $_.is_wireless } | Select-Object -First 1
    if ($usb) {
        return $usb
    }

    $wireless = $devices | Where-Object { $_.is_wireless } | Select-Object -First 1
    if ($wireless) {
        return $wireless
    }

    throw "No ADB device is online. Connect the phone over USB with USB debugging enabled, then rerun this script."
}

function Enable-WirelessAdbIfNeeded {
    param([object]$Device)
    if ($SkipWirelessAdb -or $Device.is_wireless) {
        return $Device.serial
    }

    $ip = Get-DeviceWifiAddress -Serial $Device.serial
    $target = "${ip}:$WirelessPort"
    Write-Step "enable wireless adb on $($Device.serial), target $target"
    Invoke-AdbCommand -Serial $Device.serial -AdbArgs @("tcpip", [string]$WirelessPort) | Out-Host
    Start-Sleep -Seconds 2
    Connect-WirelessDevice -Target $target

    $devices = @(Get-AdbDevices | Where-Object { $_.state -eq "device" })
    if (!($devices | Where-Object { $_.serial -eq $target })) {
        throw "Wireless ADB target did not come online after adb tcpip: $target"
    }
    return $target
}

function Invoke-ApkMcpTool {
    param(
        [string]$Serial,
        [string]$Tool,
        [object]$Arguments = @{},
        [int]$RequestTimeoutSec = 120,
        [switch]$EnsureMainActivity
    )
    $json = ConvertTo-JsonCompact $Arguments
    $cmd = @{
        Adb = $Adb
        DeviceSerial = $Serial
        Tool = $Tool
        Arguments = $json
        HealthTimeoutSec = 8
        RequestTimeoutSec = $RequestTimeoutSec
    }
    if ($EnsureMainActivity) {
        $cmd.EnsureMainActivity = $true
    }
    return & $InvokeMcpScript @cmd
}

function Get-McpStructuredContent {
    param([object]$Response)
    if ($null -eq $Response) {
        return $null
    }
    if ($Response.PSObject.Properties["result"] -and $Response.result.PSObject.Properties["structuredContent"]) {
        return $Response.result.structuredContent
    }
    if ($Response.PSObject.Properties["structuredContent"]) {
        return $Response.structuredContent
    }
    return $null
}

function Assert-McpOk {
    param(
        [object]$Response,
        [string]$Label
    )
    if ($Response.PSObject.Properties["error"]) {
        throw "$Label returned JSON-RPC error: $(ConvertTo-JsonCompact $Response.error)"
    }
    if ($Response.PSObject.Properties["result"] -and $Response.result.PSObject.Properties["isError"] -and $Response.result.isError) {
        throw "$Label returned MCP tool error: $(ConvertTo-JsonCompact $Response.result)"
    }
}

function New-ChatProbeArguments {
    param(
        [string]$TraceId,
        [string]$ConversationId,
        [string]$Message,
        [bool]$IsDevelopment,
        [string]$WaitFor = "first_reply",
        [int]$WaitTimeoutSec = 90
    )
    return [ordered]@{
        message = $Message
        project_id = $ProjectId
        project_title = $ProjectTitle
        conversation_id = $ConversationId
        conversation_title = $ConversationId
        trace_id = $TraceId
        is_development = $IsDevelopment
        wait_for = $WaitFor
        wait_timeout_ms = ([Math]::Min(120, [Math]::Max(1, $WaitTimeoutSec)) * 1000)
        poll_interval_ms = 250
        include_diagnostic_bundle = $false
        include_network_check = $false
        include_server_trace = $true
        server_trace_limit = 160
        timeline_limit = 120
    }
}

function Wait-TraceDone {
    param(
        [string]$Serial,
        [string]$TraceId,
        [int]$TimeoutSec
    )
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Max(1, $TimeoutSec))
    $lastStatus = $null
    do {
        $statusResponse = Invoke-ApkMcpTool -Serial $Serial -Tool "task_status" -Arguments ([ordered]@{
            trace_id = $TraceId
            include_events = $true
            event_limit = 120
        }) -RequestTimeoutSec 30
        Assert-McpOk -Response $statusResponse -Label "task_status $TraceId"
        $lastStatus = Get-McpStructuredContent $statusResponse
        $status = [string]$lastStatus.status
        if ($status -in @("done", "error", "cancelled", "interrupted")) {
            return $lastStatus
        }
        Start-Sleep -Seconds ([Math]::Max(1, $PollIntervalSec))
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    throw "Timed out waiting for trace $TraceId to finish. Last status: $(ConvertTo-JsonCompact $lastStatus)"
}

function Assert-TraceDone {
    param(
        [object]$Status,
        [string]$TraceId
    )
    if ([string]$Status.status -ne "done") {
        throw "Trace $TraceId did not finish successfully: $(ConvertTo-JsonCompact $Status)"
    }
}

function Assert-TraceReplyContains {
    param(
        [object]$Status,
        [string]$TraceId,
        [string]$Needle
    )
    $preview = [string]$Status.last_message_preview
    if (!$preview.Contains($Needle)) {
        throw "Trace $TraceId final reply did not contain '$Needle'. Preview: $preview"
    }
}

function Invoke-GitChecked {
    param([string[]]$GitArgs)
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & git -C $RepoRoot @GitArgs 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed ($exitCode): $(($output | Out-String).Trim())"
    }
    return $output
}

function Assert-GitProbePushed {
    param(
        [string]$DocPath,
        [string]$RunId
    )
    Invoke-GitChecked -GitArgs @("fetch", "origin", "main") | Out-Null
    Invoke-GitChecked -GitArgs @("cat-file", "-e", "origin/main:$DocPath") | Out-Null

    $grep = "--grep=test(mcp): verify native apk git path $RunId"
    $commit = (Invoke-GitChecked -GitArgs @("log", "-1", "--format=%H", $grep, "origin/main", "--", $DocPath) |
        Select-Object -First 1)
    $commit = ([string]$commit).Trim()
    if (!$commit) {
        throw "Git probe did not push expected commit for $DocPath on origin/main."
    }
    return $commit
}

function Wait-ServerGitSha {
    param(
        [string]$ExpectedSha,
        [int]$TimeoutSec
    )
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Max(1, $TimeoutSec))
    $lastVersion = $null
    $lastHealth = $null
    do {
        try {
            $health = Invoke-WebRequest -Uri "$ServerBaseUrl/health" -UseBasicParsing -TimeoutSec 15
            $lastHealth = [ordered]@{
                status_code = [int]$health.StatusCode
                body = ([string]$health.Content).Trim()
            }
            $version = Invoke-RestMethod -Uri "$ServerBaseUrl/api/server/version" -TimeoutSec 15
            $lastVersion = $version
            if ([string]$version.gitSha -eq $ExpectedSha) {
                return [ordered]@{
                    ok = $true
                    expected_git_sha = $ExpectedSha
                    health = $lastHealth
                    version = $version
                }
            }
        } catch {
            $lastVersion = [ordered]@{
                error = $_.Exception.Message
            }
        }
        Start-Sleep -Seconds ([Math]::Max(1, $PollIntervalSec))
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    throw "Server publish did not reach gitSha $ExpectedSha. Last health: $(ConvertTo-JsonCompact $lastHealth). Last version: $(ConvertTo-JsonCompact $lastVersion)"
}

function Start-ChatProbe {
    param(
        [string]$Serial,
        [string]$TraceId,
        [string]$ConversationId,
        [string]$Message,
        [bool]$IsDevelopment,
        [string]$WaitFor = "first_reply",
        [int]$WaitTimeoutSec = 90
    )
    Write-Step "chat_probe $TraceId wait_for=$WaitFor development=$IsDevelopment"
    $args = New-ChatProbeArguments `
        -TraceId $TraceId `
        -ConversationId $ConversationId `
        -Message $Message `
        -IsDevelopment $IsDevelopment `
        -WaitFor $WaitFor `
        -WaitTimeoutSec $WaitTimeoutSec
    $response = Invoke-ApkMcpTool -Serial $Serial -Tool "chat_probe" -Arguments $args -RequestTimeoutSec ($WaitTimeoutSec + 20)
    Assert-McpOk -Response $response -Label "chat_probe $TraceId"
    return Get-McpStructuredContent $response
}

$runId = "{0}_{1}" -f $ConversationPrefix, (Get-Date -Format "yyyyMMdd_HHmmss")
$conversationId = $runId
$newTrace = "${runId}_new"
$contextTrace = "${runId}_context"
$gitTrace = "${runId}_git"

$summary = [ordered]@{
    ok = $false
    run_id = $runId
    adb_serial = $null
    project_id = $ProjectId
    project_title = $ProjectTitle
    traces = [ordered]@{}
}

try {
    $selected = Resolve-AdbDevice
    $effectiveSerial = Enable-WirelessAdbIfNeeded -Device $selected
    $summary.adb_serial = $effectiveSerial
    Write-Step "using ADB serial $effectiveSerial"

    Write-Step "phone_status"
    $phoneStatus = Invoke-ApkMcpTool -Serial $effectiveSerial -Tool "phone_status" -RequestTimeoutSec 20
    Assert-McpOk -Response $phoneStatus -Label "phone_status"
    $summary.phone_status = Get-McpStructuredContent $phoneStatus

    Write-Step "ui_control state"
    $uiState = Invoke-ApkMcpTool `
        -Serial $effectiveSerial `
        -Tool "ui_control" `
        -Arguments ([ordered]@{ action = "state" }) `
        -RequestTimeoutSec 30 `
        -EnsureMainActivity
    Assert-McpOk -Response $uiState -Label "ui_control state"
    $summary.ui_control = Get-McpStructuredContent $uiState

    $newProbe = Start-ChatProbe `
        -Serial $effectiveSerial `
        -TraceId $newTrace `
        -ConversationId $conversationId `
        -Message "Reply only with marker $newTrace. Your whole response must be exactly $newTrace." `
        -IsDevelopment $false `
        -WaitFor "first_reply" `
        -WaitTimeoutSec $FirstReplyTimeoutSec
    $newStatus = Wait-TraceDone -Serial $effectiveSerial -TraceId $newTrace -TimeoutSec $FinishTimeoutSec
    Assert-TraceDone -Status $newStatus -TraceId $newTrace
    Assert-TraceReplyContains -Status $newStatus -TraceId $newTrace -Needle $newTrace
    $summary.traces.new_conversation = [ordered]@{
        trace_id = $newTrace
        probe = $newProbe
        final_status = $newStatus
    }

    $contextProbe = Start-ChatProbe `
        -Serial $effectiveSerial `
        -TraceId $contextTrace `
        -ConversationId $conversationId `
        -Message "Continue the same conversation. Reply with marker $contextTrace and include the marker from your previous reply. Do not ask me to provide it." `
        -IsDevelopment $false `
        -WaitFor "first_reply" `
        -WaitTimeoutSec $FirstReplyTimeoutSec
    $contextStatus = Wait-TraceDone -Serial $effectiveSerial -TraceId $contextTrace -TimeoutSec $FinishTimeoutSec
    Assert-TraceDone -Status $contextStatus -TraceId $contextTrace
    Assert-TraceReplyContains -Status $contextStatus -TraceId $contextTrace -Needle $contextTrace
    Assert-TraceReplyContains -Status $contextStatus -TraceId $contextTrace -Needle $newTrace
    $summary.traces.context_conversation = [ordered]@{
        trace_id = $contextTrace
        probe = $contextProbe
        final_status = $contextStatus
    }

    if ($RunGitProbe -or $RunPublishProbe) {
        $docPath = "docs/codex-mcp-native-e2e-$runId.md"
        $gitMessage = @"
You are already inside an APK native MCP-triggered Codex development task for the elon self-project.

Hard rules:
- Do not run scripts\test-apk-mcp-e2e.ps1.
- Do not run scripts\invoke-apk-mcp.ps1.
- Do not start another phone, APK, ADB, MCP, or E2E probe.
- Work directly in the current Git worktree and complete the Git operations below.

Required direct tasks:
- Create or update $docPath with trace id $gitTrace and the current timestamp.
- Run git status --short.
- Commit only that file with message: test(mcp): verify native apk git path $runId
- Push the current commit to origin main.
- Run scripts\check-task-complete.ps1 -Kind CodePushed.
"@
        if ($RunPublishProbe) {
            $gitMessage += @"
- Run scripts\publish-server.ps1 after CodePushed succeeds.
- Verify /health plus /api/server/version after publish.
"@
        } else {
            $gitMessage += @"
- Do not publish APK or server unless a required code fix is made.
"@
        }
        $gitMessage += @"

End your final reply with marker $gitTrace.
"@

        $gitProbe = Start-ChatProbe `
            -Serial $effectiveSerial `
            -TraceId $gitTrace `
            -ConversationId "${conversationId}_git" `
            -Message $gitMessage `
            -IsDevelopment $true `
            -WaitFor "first_server_event" `
            -WaitTimeoutSec $FirstReplyTimeoutSec
        $gitStatus = Wait-TraceDone -Serial $effectiveSerial -TraceId $gitTrace -TimeoutSec ([Math]::Max($FinishTimeoutSec, 1200))
        Assert-TraceDone -Status $gitStatus -TraceId $gitTrace
        $pushedCommit = Assert-GitProbePushed -DocPath $docPath -RunId $runId
        $summary.traces.git_probe = [ordered]@{
            trace_id = $gitTrace
            probe = $gitProbe
            final_status = $gitStatus
            requested_publish = [bool]$RunPublishProbe
            pushed_commit = $pushedCommit
        }
        if ($RunPublishProbe) {
            Write-Step "verify server publish gitSha $pushedCommit"
            $summary.server_publish = Wait-ServerGitSha -ExpectedSha $pushedCommit -TimeoutSec $PublishVerifyTimeoutSec
        }
    }

    $summary.ok = $true
    $jsonSummary = ConvertTo-JsonCompact $summary
    if ($ReportPath.Trim()) {
        $resolvedReport = if ([System.IO.Path]::IsPathRooted($ReportPath)) {
            $ReportPath
        } else {
            Join-Path $RepoRoot $ReportPath
        }
        $parent = Split-Path -Parent $resolvedReport
        if ($parent -and !(Test-Path -LiteralPath $parent)) {
            New-Item -ItemType Directory -Path $parent -Force | Out-Null
        }
        Set-Content -LiteralPath $resolvedReport -Value ($summary | ConvertTo-Json -Depth 60) -Encoding UTF8
        Write-Step "report written to $resolvedReport"
    }
    $jsonSummary
} catch {
    $summary.error = $_.Exception.Message
    if ($ReportPath.Trim()) {
        $resolvedReport = if ([System.IO.Path]::IsPathRooted($ReportPath)) {
            $ReportPath
        } else {
            Join-Path $RepoRoot $ReportPath
        }
        $parent = Split-Path -Parent $resolvedReport
        if ($parent -and !(Test-Path -LiteralPath $parent)) {
            New-Item -ItemType Directory -Path $parent -Force | Out-Null
        }
        Set-Content -LiteralPath $resolvedReport -Value ($summary | ConvertTo-Json -Depth 60) -Encoding UTF8
        Write-Step "failure report written to $resolvedReport"
    }
    throw
}

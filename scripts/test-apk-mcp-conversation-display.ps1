param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ProjectId = "elon-self",
    [string]$ProjectTitle = "Elon Self Project",
    [string]$ConversationPrefix = "mcp_display_e2e",
    [string]$RuntimeRoute = "",
    [string]$PreferredNodeId = "",
    [string]$PreferredWorkspacePath = "",
    [switch]$NonDevelopment,
    [int]$FinishTimeoutSec = 360,
    [int]$PollIntervalSec = 4,
    [string]$ReportPath = ""
)

$ErrorActionPreference = "Stop"

$InvokeMcpScript = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
if (!(Test-Path -LiteralPath $Adb)) {
    throw "adb not found: $Adb"
}
if (!(Test-Path -LiteralPath $InvokeMcpScript)) {
    throw "invoke-apk-mcp.ps1 not found: $InvokeMcpScript"
}

function Write-Step {
    param([string]$Message)
    Write-Host "[apk-mcp-display] $Message"
}

function ConvertTo-JsonCompact {
    param([object]$Value)
    return ($Value | ConvertTo-Json -Depth 50 -Compress)
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
            $devices += [pscustomobject]@{
                serial = $Matches[1]
                state = $Matches[2]
                detail = $Matches[3].Trim()
            }
        }
    }
    return $devices
}

function Resolve-AdbDevice {
    $online = @(Get-AdbDevices | Where-Object { $_.state -eq "device" })
    if ($DeviceSerial.Trim()) {
        $wanted = $DeviceSerial.Trim()
        $match = $online | Where-Object { $_.serial -eq $wanted } | Select-Object -First 1
        if ($match) {
            return $match.serial
        }
        throw "Requested ADB device is not online: $wanted"
    }
    if ($online.Count -eq 0) {
        throw "No ADB device is online. Connect the phone over USB or wireless ADB, then rerun this script."
    }
    return [string]$online[0].serial
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
        HealthTimeoutSec = 20
        RequestTimeoutSec = $RequestTimeoutSec
        OpenAppOnFailure = $true
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

function Get-ConversationState {
    param(
        [string]$Serial,
        [string]$ConversationId
    )
    $stateResponse = Invoke-ApkMcpTool `
        -Serial $Serial `
        -Tool "ui_control" `
        -Arguments ([ordered]@{
            action = "open_project_chat"
            project_id = $ProjectId
            conversation_id = $ConversationId
            reload_if_missing = $true
        }) `
        -RequestTimeoutSec 30 `
        -EnsureMainActivity
    Assert-McpOk -Response $stateResponse -Label "ui_control open_project_chat"
    return Get-McpStructuredContent $stateResponse
}

function Get-ConversationMessages {
    param([object]$ConversationState)
    $conversation = $ConversationState.active_conversation
    if ($null -eq $conversation) {
        throw "ui_control did not return active_conversation: $(ConvertTo-JsonCompact $ConversationState)"
    }
    return @($conversation.messages)
}

function Assert-MessageExists {
    param(
        [object[]]$Messages,
        [string]$Label,
        [scriptblock]$Predicate
    )
    foreach ($message in $Messages) {
        if (& $Predicate $message) {
            return $message
        }
    }
    throw "Missing message: $Label. Messages: $(ConvertTo-JsonCompact $Messages)"
}

function Assert-ConversationSeedVisible {
    param(
        [object]$ConversationState,
        [string]$TraceId,
        [bool]$IsDevelopment
    )
    if ([string]$ConversationState.active_conversation.id -ne $conversationId) {
        throw "Active conversation mismatch: expected $conversationId, got $($ConversationState.active_conversation.id)"
    }
    $messages = Get-ConversationMessages -ConversationState $ConversationState
    Assert-MessageExists -Messages $messages -Label "MCP user bubble" -Predicate {
        param($message)
        ([string]$message.role -eq "user") -and ([string]$message.id -eq "mcp:$TraceId:user")
    } | Out-Null
    if ($IsDevelopment) {
        Assert-MessageExists -Messages $messages -Label "Codex intent fold layer" -Predicate {
            param($message)
            ([string]$message.role -eq "ai-intent") -and
                ([string]$message.id -eq "mcp:$TraceId:intent") -and
                ![string]::IsNullOrWhiteSpace([string]$message.evidence_title)
        } | Out-Null
    }
    Assert-MessageExists -Messages $messages -Label "workflow started status" -Predicate {
        param($message)
        ([string]$message.id -eq "mcp:$TraceId:working") -or ([string]$message.role -in @("ai-working", "ai-progress", "ai-tool", "ai-cli-log"))
    } | Out-Null
}

function Wait-TraceTerminal {
    param(
        [string]$Serial,
        [string]$TraceId
    )
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Max(1, $FinishTimeoutSec))
    $lastStatus = $null
    do {
        $statusResponse = Invoke-ApkMcpTool -Serial $Serial -Tool "task_status" -Arguments ([ordered]@{
            trace_id = $TraceId
            include_events = $true
            event_limit = 160
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

function Assert-FinalReplyVisible {
    param(
        [object]$ConversationState,
        [string]$TraceId,
        [bool]$IsDevelopment
    )
    $messages = Get-ConversationMessages -ConversationState $ConversationState
    Assert-MessageExists -Messages $messages -Label "final assistant reply after MCP user message" -Predicate {
        param($message)
        ([string]$message.role -eq "ai") -and ([string]$message.content_preview -match [regex]::Escape($TraceId))
    } | Out-Null
    if ($IsDevelopment) {
        Assert-MessageExists -Messages $messages -Label "collapsible process evidence" -Predicate {
            param($message)
            ([string]$message.role -in @("ai", "ai-intent")) -and
                (![string]::IsNullOrWhiteSpace([string]$message.evidence_title) -or
                    ![string]::IsNullOrWhiteSpace([string]$message.evidence_details_preview))
        } | Out-Null
    }
}

$runId = "$ConversationPrefix`_$((Get-Date).ToUniversalTime().ToString('yyyyMMdd_HHmmss'))"
$traceId = "${runId}_chat"
$conversationId = "${runId}_conversation"
$isDevelopment = !$NonDevelopment
$message = if ($isDevelopment) {
    "APK native MCP conversation display verification. Do read-only status checks only. Do not edit files, commit, push, publish, or release. The final reply must contain marker $traceId."
} else {
    "Reply with marker $traceId. Your final reply must contain $traceId."
}

$summary = [ordered]@{
    ok = $false
    run_id = $runId
    trace_id = $traceId
    conversation_id = $conversationId
    is_development = $isDevelopment
    adb_serial = $null
}

try {
    $serial = Resolve-AdbDevice
    $summary.adb_serial = $serial
    Write-Step "using ADB serial $serial"

    Write-Step "phone_status"
    $phoneStatus = Invoke-ApkMcpTool -Serial $serial -Tool "phone_status" -RequestTimeoutSec 20
    Assert-McpOk -Response $phoneStatus -Label "phone_status"
    $summary.phone_status = Get-McpStructuredContent $phoneStatus

    $chatArgs = [ordered]@{
        message = $message
        project_id = $ProjectId
        project_title = $ProjectTitle
        conversation_id = $conversationId
        conversation_title = "MCP Display $runId"
        trace_id = $traceId
        is_development = $isDevelopment
        show_in_ui = $true
        start_ack_timeout_ms = 5000
    }
    if ($RuntimeRoute.Trim()) {
        $chatArgs.runtimeRoute = $RuntimeRoute.Trim()
    }
    if ($isDevelopment) {
        $chatArgs.execution_mode = "execute"
        $chatArgs.plan_mode = $false
    }
    if ($PreferredNodeId.Trim()) {
        $chatArgs.local_node_id = $PreferredNodeId.Trim()
    }
    if ($PreferredWorkspacePath.Trim()) {
        $chatArgs.local_workspace_path = $PreferredWorkspacePath.Trim()
    }

    Write-Step "chat_send $traceId"
    $chatSend = Invoke-ApkMcpTool -Serial $serial -Tool "chat_send" -Arguments $chatArgs -RequestTimeoutSec 40 -EnsureMainActivity
    Assert-McpOk -Response $chatSend -Label "chat_send"
    $summary.chat_send = Get-McpStructuredContent $chatSend

    Write-Step "assert seeded conversation visible"
    $seedState = Get-ConversationState -Serial $serial -ConversationId $conversationId
    Assert-ConversationSeedVisible -ConversationState $seedState -TraceId $traceId -IsDevelopment $isDevelopment
    $summary.seed_state = $seedState

    Write-Step "wait task terminal"
    $terminal = Wait-TraceTerminal -Serial $serial -TraceId $traceId
    if ([string]$terminal.status -ne "done") {
        throw "Trace $traceId did not finish successfully: $(ConvertTo-JsonCompact $terminal)"
    }
    $summary.terminal_status = $terminal

    Write-Step "assert final reply and fold evidence visible"
    $finalState = Get-ConversationState -Serial $serial -ConversationId $conversationId
    Assert-FinalReplyVisible -ConversationState $finalState -TraceId $traceId -IsDevelopment $isDevelopment
    $summary.final_state = $finalState
    $summary.ok = $true
} finally {
    if (!$ReportPath.Trim()) {
        $ReportPath = Join-Path $PSScriptRoot "..\target\apk-mcp-conversation-display-$runId.json"
    }
    $reportPathForResolve = if ([System.IO.Path]::IsPathRooted($ReportPath)) {
        $ReportPath
    } else {
        Join-Path (Get-Location) $ReportPath
    }
    $reportFullPath = [System.IO.Path]::GetFullPath($reportPathForResolve)
    $reportDir = Split-Path -Parent $reportFullPath
    if (!(Test-Path -LiteralPath $reportDir)) {
        New-Item -ItemType Directory -Path $reportDir | Out-Null
    }
    $summary | ConvertTo-Json -Depth 60 | Set-Content -LiteralPath $reportFullPath -Encoding UTF8
    Write-Step "report $reportFullPath"
}

if (!$summary.ok) {
    throw "APK MCP conversation display verification failed. Report: $ReportPath"
}

Write-Host "APK MCP conversation display verification passed: $traceId"

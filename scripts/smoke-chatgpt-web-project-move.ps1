#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [ValidateRange(30, 300)][int]$TimeoutSec = 150,
    [switch]$ConfirmRoundTrip
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion `
    -ExpectedAdapterVersion $ExpectedAdapterVersion

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$originPath = ""
$candidate = $null
$originProject = $null
$targetProject = $null
$forwardWriteSelected = $false
$forwardMoveVerified = $false
$restoreWriteSelected = $false
$restored = $false
$recoveryUnknown = $false
$primaryFailure = $null
$cleanupFailure = $null

function Invoke-MainAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{}
    )

    return Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments -EnsureMainActivity
}

function Wait-ProductionReady {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
        $chat = $state.social_chat
        $web = $state.chatgpt_web_mcp
        if (
            [string]$state.active_surface -eq "social_ai" -and
            [string]$chat.interaction_mode -eq "chat" -and
            [string]$chat.web_chat_provider_id -eq "chatgpt_web" -and
            [string]$chat.web_chat_state -eq "ready" -and
            $chat.web_chat_composer_ready -eq $true -and
            [string]$web.bridge_state -eq "ready" -and
            $web.adapter_current -eq $true -and
            $web.authenticated -eq $true -and
            [int]$web.adapter_version -eq $ExpectedAdapterVersion
        ) {
            return $state
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "The production ChatGPT surface did not become ready for project-move acceptance."
}

function Get-Navigation {
    $conversations = [System.Collections.Generic.List[object]]::new()
    $projects = [System.Collections.Generic.List[object]]::new()
    $offset = 0
    do {
        $page = Invoke-MainAction -Action "get_web_chat_navigation" -Arguments @{
            offset = $offset
            limit = 50
        }
        if ([string]$page.schema -ne "elon.web_chat.navigation.v1") {
            throw "Native ChatGPT navigation cache is unavailable."
        }
        @($page.conversations).ForEach({ $conversations.Add($_) })
        @($page.projects).ForEach({ $projects.Add($_) })
        $hasMore = $page.conversation_has_more -eq $true -or $page.project_has_more -eq $true
        $offset += 50
    } while ($hasMore -and $offset -le 500)
    return [pscustomobject]@{
        conversations = @($conversations)
        projects = @($projects)
    }
}

function Get-UiNodes {
    $remotePath = "/sdcard/elon-chatgpt-project-move.xml"
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "uiautomator", "dump", $remotePath) `
            -TimeoutSec 30 -Label "dump native project-move selectors" | Out-Null
        $raw = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "cat", $remotePath) `
            -TimeoutSec 30 -Label "read native project-move selectors"
        $document = [xml]$raw
        return @($document.SelectNodes("//node"))
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remotePath) `
            -TimeoutSec 5 -Label "remove native project-move selector dump" | Out-Null
    }
}

function ConvertTo-NativeToken {
    param([AllowEmptyString()][string]$Value)

    $token = ($Value.Trim() -replace '[^A-Za-z0-9_.-]', '_')
    if ($token.Length -gt 96) { $token = $token.Substring(0, 96) }
    if (-not $token) { return "unknown" }
    return $token
}

function Invoke-NativeSelector {
    param(
        [Parameter(Mandatory = $true)][string]$Selector,
        [Parameter(Mandatory = $true)][string]$Stage,
        [switch]$Prefix,
        [switch]$Optional
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(12)
    do {
        $node = @(Get-UiNodes | Where-Object {
            $contentDescription = [string]$_.GetAttribute("content-desc")
            if ($Prefix) {
                $contentDescription.StartsWith($Selector, [StringComparison]::Ordinal)
            } else {
                $contentDescription -eq $Selector
            }
        }) | Select-Object -First 1
        if ($null -ne $node) {
            $bounds = [regex]::Match(
                [string]$node.GetAttribute("bounds"),
                '^\[(\d+),(\d+)\]\[(\d+),(\d+)\]$'
            )
            if (-not $bounds.Success) { throw "Native selector returned invalid bounds." }
            $x = [int](([int]$bounds.Groups[1].Value + [int]$bounds.Groups[3].Value) / 2)
            $y = [int](([int]$bounds.Groups[2].Value + [int]$bounds.Groups[4].Value) / 2)
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "tap", "$x", "$y") `
                -TimeoutSec 5 -Label "invoke native project-move selector" | Out-Null
            Start-Sleep -Milliseconds 500
            return $true
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $visibleDescriptions = @(Get-UiNodes | ForEach-Object {
        [string]$_.GetAttribute("content-desc")
    })
    $actionControlCount = @($visibleDescriptions | Where-Object {
        $_ -in @(
            "web-chat-conversation-action-move-to-project",
            "web-chat-conversation-action-more-settings",
            "web-chat-conversation-actions-official"
        )
    }).Count
    $destinationCount = @($visibleDescriptions | Where-Object {
        $_ -like "web-chat-conversation-project-destination:*"
    }).Count
    if ($Optional) { return $false }
    throw (
        "A required native project-move selector was not visible at stage: " +
            "$Stage (action_controls=$actionControlCount, destinations=$destinationCount)."
    )
}

function Open-ProjectSidebar {
    param([Parameter(Mandatory = $true)][string]$ProjectId)

    Invoke-MainAction -Action "open_chat_side_menu" | Out-Null
    Invoke-MainAction -Action "set_web_chat_sidebar" -Arguments @{
        section = "projects"
        project_id = $ProjectId
    } | Out-Null
    Start-Sleep -Milliseconds 800
}

function Close-Sidebar {
    try { Invoke-MainAction -Action "close_chat_side_menu" | Out-Null } catch {}
}

function Find-VisibleProjectConversation {
    param([Parameter(Mandatory = $true)]$Navigation)

    foreach ($project in @($Navigation.projects)) {
        $projectId = [string]$project.id
        if (-not $projectId) { continue }
        Open-ProjectSidebar -ProjectId $projectId
        $visibleTokens = @(Get-UiNodes | ForEach-Object {
            [string]$_.GetAttribute("content-desc")
        } | Where-Object {
            $_ -like "chatgpt-conversation-actions:*"
        } | ForEach-Object {
            $match = [regex]::Match($_, '^chatgpt-conversation-actions:([^:]+):')
            if ($match.Success) { $match.Groups[1].Value }
        })
        foreach ($visibleToken in $visibleTokens) {
            $matches = @($Navigation.conversations | Where-Object {
                [string]$_.project_id -eq $projectId -and
                    (ConvertTo-NativeToken -Value ([string]$_.id)) -eq $visibleToken
            })
            if ($matches.Count -eq 1) {
                return [pscustomobject]@{
                    conversation = $matches[0]
                    project = $project
                }
            }
        }
        Close-Sidebar
    }
    throw "No visible project conversation is available for reversible acceptance."
}

function Test-ProjectMoveWriteObserved {
    try {
        return @((Get-UiNodes) | Where-Object {
            $text = [string]$_.text
            $text -eq "正在提交一次移动操作" -or
                $text -eq "正在同步会话目录" -or
                $text.Contains("已经提交过一次操作")
        }).Count -gt 0
    } catch {
        return $false
    }
}

function Wait-ConversationMembership {
    param(
        [Parameter(Mandatory = $true)][string]$ConversationId,
        [Parameter(Mandatory = $true)][string]$ProjectId,
        [Parameter(Mandatory = $true)][ref]$WriteSelected
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        if (-not $WriteSelected.Value -and (Test-ProjectMoveWriteObserved)) {
            $WriteSelected.Value = $true
        }
        $navigation = Get-Navigation
        $current = @($navigation.conversations | Where-Object {
            [string]$_.id -eq $ConversationId
        }) | Select-Object -First 1
        if ($null -ne $current -and [string]$current.project_id -eq $ProjectId) {
            $WriteSelected.Value = $true
            return $current
        }
        Start-Sleep -Seconds 2
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Native navigation did not reconcile the project move."
}

function Invoke-ProjectMove {
    param(
        [Parameter(Mandatory = $true)]$Conversation,
        [Parameter(Mandatory = $true)][string]$SourceProjectId,
        [Parameter(Mandatory = $true)][string]$DestinationProjectId,
        [Parameter(Mandatory = $true)][ref]$WriteSelected
    )

    $conversationToken = ConvertTo-NativeToken -Value ([string]$Conversation.id)
    $pickerOpened = $false
    for ($attempt = 1; $attempt -le 2; $attempt++) {
        Open-ProjectSidebar -ProjectId $SourceProjectId
        $null = Invoke-NativeSelector `
            -Selector ("chatgpt-conversation-actions:" + $conversationToken + ":") `
            -Stage "conversation-actions" `
            -Prefix
        $pickerOpened = Invoke-NativeSelector `
            -Selector "web-chat-conversation-action-move-to-project" `
            -Stage "move-action" `
            -Optional
        if ($pickerOpened) { break }
        Close-Sidebar
        Start-Sleep -Milliseconds 1200
    }
    if (-not $pickerOpened) {
        throw "The native move-to-project picker did not open after one safe retry."
    }
    $null = Invoke-NativeSelector -Selector (
        "web-chat-conversation-project-destination:" + $DestinationProjectId
    ) -Stage "project-destination"
    return Wait-ConversationMembership -ConversationId ([string]$Conversation.id) `
        -ProjectId $DestinationProjectId -WriteSelected $WriteSelected
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
    $ready = Wait-ProductionReady
    $originPath = [string]$ready.social_chat.web_chat_conversation_path
    $navigation = Get-Navigation
    if (@($navigation.projects).Count -lt 2) {
        throw "At least two cached projects are required for reversible acceptance."
    }
    $sample = Find-VisibleProjectConversation -Navigation $navigation
    $candidate = $sample.conversation
    $originProject = $sample.project
    $targetProject = @($navigation.projects | Where-Object {
        [string]$_.id -ne [string]$originProject.id
    }) | Select-Object -First 1
    if ($null -eq $targetProject) { throw "No alternate project is available."
    }

    if ($ConfirmRoundTrip) {
        $moved = Invoke-ProjectMove -Conversation $candidate `
            -SourceProjectId ([string]$originProject.id) `
            -DestinationProjectId ([string]$targetProject.id) `
            -WriteSelected ([ref]$forwardWriteSelected)
        $forwardMoveVerified = $true
        $restoredConversation = Invoke-ProjectMove -Conversation $moved `
            -SourceProjectId ([string]$targetProject.id) `
            -DestinationProjectId ([string]$originProject.id) `
            -WriteSelected ([ref]$restoreWriteSelected)
        $restored = [string]$restoredConversation.project_id -eq [string]$originProject.id
    }

    Close-Sidebar
    if ($originPath) {
        Invoke-MainAction -Action "open_web_chat_conversation" -Arguments @{
            conversation_path = $originPath
        } | Out-Null
    }
    [ordered]@{
        schema = "elon.chatgpt_web.project_move_smoke.v1"
        passed = (-not $ConfirmRoundTrip) -or ($forwardMoveVerified -and $restored)
        adapter_version = $ExpectedAdapterVersion
        cached_project_count = @($navigation.projects).Count
        cached_conversation_count = @($navigation.conversations).Count
        native_selector_verified = $true
        round_trip_requested = [bool]$ConfirmRoundTrip
        forward_move_verified = $forwardMoveVerified
        original_membership_restored = $restored
        writes_invoked = if ($ConfirmRoundTrip) { 2 } else { 0 }
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 4
    Write-Output "CHATGPT_WEB_PROJECT_MOVE_STATUS=passed"
} catch {
    $primaryFailure = $_
} finally {
    if ($forwardWriteSelected -and -not $restored -and $null -ne $candidate) {
        try {
            $recoveryNavigation = Get-Navigation
            $current = @($recoveryNavigation.conversations | Where-Object {
                [string]$_.id -eq [string]$candidate.id
            }) | Select-Object -First 1
            $currentProjectId = [string]$current.project_id
            if (
                $currentProjectId -eq [string]$targetProject.id -and
                -not $restoreWriteSelected
            ) {
                $recoveryWriteSelected = $false
                $recovered = Invoke-ProjectMove -Conversation $current `
                    -SourceProjectId $currentProjectId `
                    -DestinationProjectId ([string]$originProject.id) `
                    -WriteSelected ([ref]$recoveryWriteSelected)
                $restored = [string]$recovered.project_id -eq [string]$originProject.id
            } elseif (
                $currentProjectId -eq [string]$originProject.id -and
                ($forwardMoveVerified -or $restoreWriteSelected)
            ) {
                $restored = $true
            } else {
                $recoveryUnknown = $true
            }
        } catch {
            $recoveryUnknown = $true
        }
    }
    try { Close-Sidebar } catch { $cleanupFailure = $_ }
    try {
        if ($originPath) {
            Invoke-MainAction -Action "open_web_chat_conversation" -Arguments @{
                conversation_path = $originPath
            } | Out-Null
        }
    } catch {}
    try {
        Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    } catch {
        if ($null -eq $cleanupFailure) { $cleanupFailure = $_ }
    }
}

if ($recoveryUnknown) {
    $recoveryDetail = "Project-move recovery is ambiguous; inspect the official project menu before retrying."
    if ($null -ne $primaryFailure) {
        throw ($primaryFailure.Exception.Message + " " + $recoveryDetail)
    }
    throw $recoveryDetail
}
if ($null -ne $primaryFailure) { throw $primaryFailure }
if ($null -ne $cleanupFailure) { throw $cleanupFailure }

#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [ValidateRange(30, 180)][int]$TimeoutSec = 150,
    [ValidateRange(0, 39)][int]$TargetProjectOffset = 0,
    [switch]$ConfirmRoundTrip
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-project-move-write-observation.ps1")
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
$cleanupWriteSelected = $false
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
        -Arguments $Arguments
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

function Ensure-ProductionReady {
    try {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
        if (
            [string]$state.active_surface -eq "social_ai" -and
            [string]$state.social_chat.interaction_mode -eq "chat" -and
            [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
            [string]$state.social_chat.web_chat_state -eq "ready" -and
            $state.social_chat.web_chat_composer_ready -eq $true
        ) {
            return $state
        }
    } catch {}
    Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
    return Wait-ProductionReady
}

function Invoke-ReadActionWithSurfaceRecovery {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{}
    )

    try {
        return Invoke-MainAction -Action $Action -Arguments $Arguments
    } catch {
        if (
            [string]$_.Exception.Message -notmatch
                "web_chat_mode_inactive|web_chat_not_ready|main_activity_not_bound"
        ) {
            throw
        }
        Ensure-ProductionReady | Out-Null
        return Invoke-MainAction -Action $Action -Arguments $Arguments
    }
}

function Get-Navigation {
    $conversations = [System.Collections.Generic.List[object]]::new()
    $projects = [System.Collections.Generic.List[object]]::new()
    $offset = 0
    do {
        $page = Invoke-ReadActionWithSurfaceRecovery `
            -Action "get_web_chat_navigation" -Arguments @{
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

function Request-ScopedNavigationRefresh {
    param(
        [Parameter(Mandatory = $true)][string]$ProjectId,
        [string]$ConversationPath = ""
    )

    $arguments = @{ project_id = $ProjectId }
    if ($ConversationPath.Trim()) {
        $arguments.conversation_path = $ConversationPath.Trim()
    }
    Invoke-ReadActionWithSurfaceRecovery `
        -Action "refresh_web_chat_conversations" -Arguments $arguments | Out-Null
}

function Request-MembershipRefresh {
    param(
        [Parameter(Mandatory = $true)][string]$OriginProjectId,
        [Parameter(Mandatory = $true)][string]$TargetProjectId,
        [string]$ConversationPath = ""
    )

    Request-ScopedNavigationRefresh -ProjectId $OriginProjectId `
        -ConversationPath $ConversationPath
    Start-Sleep -Seconds 2
    Request-ScopedNavigationRefresh -ProjectId $TargetProjectId `
        -ConversationPath $ConversationPath
}

function Get-UiNodes {
    $remotePath = "/data/local/tmp/elon-chatgpt-project-move-$PID.xml"
    $lastFailure = $null
    try {
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            try {
                Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                    -Arguments @("shell", "rm", "-f", $remotePath) `
                    -TimeoutSec 5 -Label "reset native project-move selector dump" | Out-Null
                Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                    -Arguments @("shell", "uiautomator", "dump", $remotePath) `
                    -TimeoutSec 30 -Label "dump native project-move selectors" | Out-Null
                $raw = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                    -Arguments @("shell", "cat", $remotePath) `
                    -TimeoutSec 30 -Label "read native project-move selectors"
                if ([string]$raw -notmatch '<hierarchy') {
                    throw "Native selector dump was not complete."
                }
                $document = [xml]$raw
                return @($document.SelectNodes("//node"))
            } catch {
                $lastFailure = $_
                if ($attempt -lt 3) { Start-Sleep -Milliseconds 400 }
            }
        }
        throw $lastFailure
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

function Get-ConversationProjectIdFromPath {
    param([AllowEmptyString()][string]$Path)

    $match = [regex]::Match(
        $Path.Trim(),
        '^/g/(g-p-[A-Za-z0-9_-]{1,160})/c/[A-Za-z0-9_-]{1,160}$'
    )
    if (-not $match.Success) { return "" }
    $value = [string]$match.Groups[1].Value
    $production = [regex]::Match($value, '^(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]+)?$')
    if ($production.Success) { return [string]$production.Groups[1].Value }
    return $value
}

function Get-ConversationIdentityFromPath {
    param([AllowEmptyString()][string]$Path)

    $match = [regex]::Match(
        $Path.Trim(),
        '^/g/g-p-[A-Za-z0-9_-]{1,160}/c/([A-Za-z0-9_-]{1,160})$'
    )
    if (-not $match.Success) { return "" }
    return [string]$match.Groups[1].Value
}

function Get-CanonicalConversationMembership {
    param(
        [Parameter(Mandatory = $true)]$Navigation,
        [Parameter(Mandatory = $true)][string]$ConversationId
    )

    $matches = @($Navigation.conversations | Where-Object {
        [string]$_.id -eq $ConversationId -and
            [string]$_.project_id -and
            (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq
                [string]$_.project_id
    })
    if ($matches.Count -ne 1) { return $null }
    return $matches[0]
}

function Get-LiveConversationMembership {
    param([Parameter(Mandatory = $true)][string]$ConversationId)

    $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
    $path = [string]$state.social_chat.web_chat_conversation_path
    if ((Get-ConversationIdentityFromPath -Path $path) -ne $ConversationId) {
        return $null
    }
    $projectId = Get-ConversationProjectIdFromPath -Path $path
    if (-not $projectId) { return $null }
    return [pscustomobject]@{
        id = $ConversationId
        path = $path
        project_id = $projectId
    }
}

function Wait-ReadOnlyOriginalMembership {
    param(
        [Parameter(Mandatory = $true)][string]$ConversationId,
        [Parameter(Mandatory = $true)][string]$ConversationPath,
        [Parameter(Mandatory = $true)][string]$OriginProjectId,
        [Parameter(Mandatory = $true)][string]$TargetProjectId
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        Request-MembershipRefresh -OriginProjectId $OriginProjectId `
            -TargetProjectId $TargetProjectId -ConversationPath $ConversationPath
        Start-Sleep -Seconds 2
        $navigation = Get-Navigation
        $current = Get-CanonicalConversationMembership `
            -Navigation $navigation -ConversationId $ConversationId
        if ($null -ne $current -and [string]$current.project_id -eq $OriginProjectId) {
            return $current
        }
        if ($null -eq $current) {
            $live = Get-LiveConversationMembership -ConversationId $ConversationId
            if ($null -ne $live -and [string]$live.project_id -eq $OriginProjectId) {
                return $live
            }
        }
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $null
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
            $visibleToUser = [string]$_.GetAttribute("visible-to-user")
            $matchesSelector = if ($Prefix) {
                $contentDescription.StartsWith($Selector, [StringComparison]::Ordinal)
            } else {
                $contentDescription -eq $Selector
            }
            $matchesSelector -and $visibleToUser -ne "false"
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
                -TimeoutSec 5 -Label "tap native project-move selector" | Out-Null
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

function Select-OfficialFallbackProject {
    param(
        [Parameter(Mandatory = $true)][string]$SourceProjectId,
        [string]$PreferredProjectId = ""
    )

    $destinationIds = @(Get-UiNodes | ForEach-Object {
        $description = [string]$_.GetAttribute("content-desc")
        $match = [regex]::Match(
            $description,
            '^web-chat-conversation-project-destination:(g-p-[A-Za-z0-9_-]{1,160})$'
        )
        if ($match.Success) { [string]$match.Groups[1].Value }
    } | Where-Object { $_ -and $_ -ne $SourceProjectId } | Select-Object -Unique)
    if ($destinationIds.Count -eq 0) { return $null }
    $navigation = Get-Navigation
    $projects = @($navigation.projects | Where-Object {
        [string]$_.id -in $destinationIds
    })
    $selected = @($projects | Sort-Object @{
        Expression = { if ([string]$_.id -eq $PreferredProjectId) { 0 } else { 1 } }
    }) | Select-Object -First 1
    if ($null -eq $selected) { return $null }
    $pressed = Invoke-NativeSelector -Selector (
        "web-chat-conversation-project-destination:" + [string]$selected.id
    ) -Stage "official-project-destination" -Optional
    if (-not $pressed) { return $null }
    return $selected
}

function Open-ProjectSidebar {
    param([Parameter(Mandatory = $true)][string]$ProjectId)

    for ($attempt = 1; $attempt -le 2; $attempt++) {
        Ensure-ProductionReady | Out-Null
        Invoke-ReadActionWithSurfaceRecovery -Action "open_chat_side_menu" | Out-Null
        Invoke-ReadActionWithSurfaceRecovery -Action "set_web_chat_sidebar" -Arguments @{
            section = "projects"
            project_id = $ProjectId
        } | Out-Null
        Start-Sleep -Milliseconds 800
        try {
            $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
            if (
                [string]$state.active_surface -eq "social_ai" -and
                [string]$state.social_chat.web_chat_state -eq "ready" -and
                $state.social_chat.web_chat_composer_ready -eq $true
            ) {
                return
            }
        } catch {}
        Wait-ProductionReady | Out-Null
    }
    throw "The project sidebar did not settle after its background refresh."
}

function Close-Sidebar {
    try { Invoke-MainAction -Action "close_chat_side_menu" | Out-Null } catch {}
}

function Find-VisibleProjectConversation {
    param([Parameter(Mandatory = $true)]$Navigation)

    $orderedProjects = @($Navigation.projects | Sort-Object @{
        Expression = { if ($_.active -eq $true) { 0 } else { 1 } }
    })
    foreach ($project in $orderedProjects) {
        $projectId = [string]$project.id
        if (-not $projectId) { continue }
        $expectedByToken = @{}
        @($Navigation.conversations | Where-Object {
            [string]$_.project_id -eq $projectId -and
                (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq $projectId
        }) | Group-Object {
            ConvertTo-NativeToken -Value ([string]$_.id)
        } | Where-Object {
            $_.Count -eq 1
        } | ForEach-Object {
            $expectedByToken[[string]$_.Name] = $_.Group[0]
        }
        if ($expectedByToken.Count -eq 0) { continue }

        Open-ProjectSidebar -ProjectId $projectId
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds(12)
        do {
            $visibleTokens = @(Get-UiNodes | ForEach-Object {
                [string]$_.GetAttribute("content-desc")
            } | Where-Object {
                $_ -like "chatgpt-conversation-actions:*"
            } | ForEach-Object {
                $match = [regex]::Match($_, '^chatgpt-conversation-actions:([^:]+):')
                if ($match.Success) { $match.Groups[1].Value }
            })
            $orderedVisibleTokens = @($visibleTokens | Sort-Object @{
                Expression = {
                    if (
                        $expectedByToken.ContainsKey([string]$_) -and
                        $expectedByToken[[string]$_].active -eq $true
                    ) { 0 } else { 1 }
                }
            })
            foreach ($visibleToken in $orderedVisibleTokens) {
                if (-not $expectedByToken.ContainsKey($visibleToken)) { continue }
                return [pscustomobject]@{
                    conversation = $expectedByToken[$visibleToken]
                    project = $project
                }
            }
            Start-Sleep -Milliseconds 500
        } while ([DateTimeOffset]::UtcNow -lt $deadline)
        Close-Sidebar
    }
    throw "No visible project conversation is available for reversible acceptance."
}

function Get-ProjectMoveUiStage {
    $texts = @(Get-UiNodes | ForEach-Object { [string]$_.text })
    if (@($texts | Where-Object { $_.Contains("当前输入框有未发送内容") }).Count -gt 0) {
        return "draft_blocked"
    }
    if (@($texts | Where-Object { $_.Contains("已经提交过一次操作") }).Count -gt 0) {
        return "failed_after_write"
    }
    if (@($texts | Where-Object { $_.Contains("尚未提交移动操作") }).Count -gt 0) {
        return "failed_before_write"
    }
    if (@($texts | Where-Object { $_ -like "已移动到*" }).Count -gt 0) {
        return "completed"
    }
    if ("正在同步会话目录" -in $texts) { return "syncing" }
    if ("正在确认移动" -in $texts) { return "confirming" }
    if ("正在提交一次移动操作" -in $texts) { return "submitting" }
    if ("正在打开项目列表" -in $texts) { return "opening_project_list" }
    if ("正在打开会话设置" -in $texts) { return "opening_conversation_options" }
    if ("正在切换到该会话" -in $texts) { return "opening_conversation" }
    if ("正在准备当前会话" -in $texts) { return "preparing" }
    return "idle"
}

function Dismiss-StalePreWriteMoveFailure {
    if ((Get-ProjectMoveUiStage) -ne "failed_before_write") { return }
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") `
        -TimeoutSec 5 -Label "dismiss stale pre-write project-move failure" | Out-Null
    Start-Sleep -Seconds 1
    if ((Get-ProjectMoveUiStage) -eq "failed_before_write") {
        throw "The stale pre-write move failure did not close."
    }
}

function Wait-ConversationMembership {
    param(
        [Parameter(Mandatory = $true)][string]$ConversationId,
        [Parameter(Mandatory = $true)][string]$ConversationPath,
        [Parameter(Mandatory = $true)][string]$SourceProjectId,
        [Parameter(Mandatory = $true)][ref]$DestinationProject,
        [Parameter(Mandatory = $true)][ref]$WriteSelected,
        [Parameter(Mandatory = $true)][long]$WriteObservationStartMs,
        [switch]$AllowFallbackDestination
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $poll = 0
    $lastUiStage = ""
    $fallbackSelected = $false
    do {
        $projectId = [string]$DestinationProject.Value.id
        $uiStage = Get-ProjectMoveUiStage
        if ($uiStage -ne $lastUiStage) {
            Write-Host "CHATGPT_WEB_PROJECT_MOVE_PROGRESS=$uiStage"
            $lastUiStage = $uiStage
        }
        if (
            $AllowFallbackDestination -and
            -not $WriteSelected.Value -and
            -not $fallbackSelected -and
            $uiStage -eq "idle"
        ) {
            $fallback = Select-OfficialFallbackProject `
                -SourceProjectId $SourceProjectId -PreferredProjectId $projectId
            if ($null -ne $fallback) {
                $DestinationProject.Value = $fallback
                $fallbackSelected = $true
                Write-Host "CHATGPT_WEB_PROJECT_MOVE_PROGRESS=official_destination_selected"
                Start-Sleep -Milliseconds 700
                continue
            }
        }
        if ($uiStage -eq "failed_before_write") {
            throw "Native project move failed before the official write was submitted."
        }
        if ($uiStage -eq "draft_blocked") {
            throw "Native project move preserved an unsent draft before any official write."
        }
        if ($uiStage -eq "failed_after_write") {
            $WriteSelected.Value = $true
            throw "Native project move requires read-only recovery after one submitted write."
        }
        if ($poll -eq 0 -or $poll % 4 -eq 0) {
            Request-ScopedNavigationRefresh -ProjectId $ProjectId `
                -ConversationPath $ConversationPath
        }
        if (-not $WriteSelected.Value -and $uiStage -in @(
                "submitting",
                "syncing",
                "completed"
            )
        ) {
            $WriteSelected.Value = $true
        }
        if (
            -not $WriteSelected.Value -and
            ($poll -eq 0 -or $poll % 4 -eq 0) -and
            (Test-ProjectMoveWriteObserved -SinceWallTimeMs $WriteObservationStartMs)
        ) {
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
        $poll += 1
        Start-Sleep -Seconds 2
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Native navigation did not reconcile the project move."
}

function Invoke-ProjectMove {
    param(
        [Parameter(Mandatory = $true)]$Conversation,
        [Parameter(Mandatory = $true)][string]$SourceProjectId,
        [Parameter(Mandatory = $true)][ref]$DestinationProject,
        [Parameter(Mandatory = $true)][ref]$WriteSelected,
        [switch]$AllowFallbackDestination
    )

    $destinationProjectId = [string]$DestinationProject.Value.id
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
    $writeObservationStartMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $null = Invoke-NativeSelector -Selector (
        "web-chat-conversation-project-destination:" + $destinationProjectId
    ) -Stage "project-destination"
    return Wait-ConversationMembership -ConversationId ([string]$Conversation.id) `
        -ConversationPath ([string]$Conversation.path) `
        -SourceProjectId $SourceProjectId -DestinationProject $DestinationProject `
        -WriteSelected $WriteSelected -WriteObservationStartMs $writeObservationStartMs `
        -AllowFallbackDestination:$AllowFallbackDestination
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
    $ready = Wait-ProductionReady
    Dismiss-StalePreWriteMoveFailure
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
    }) | Select-Object -Skip $TargetProjectOffset -First 1
    if ($null -eq $targetProject) { throw "No alternate project is available."
    }

    Request-MembershipRefresh -OriginProjectId ([string]$originProject.id) `
        -TargetProjectId ([string]$targetProject.id) `
        -ConversationPath ([string]$candidate.path)
    Start-Sleep -Seconds 2
    $navigation = Get-Navigation
    $candidateMemberships = @($navigation.conversations | Where-Object {
        [string]$_.id -eq [string]$candidate.id -and
            [string]$_.project_id -and
            (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq
                [string]$_.project_id
    })
    $candidate = @($candidateMemberships | Where-Object {
        [string]$_.id -eq [string]$candidate.id -and
            [string]$_.project_id -eq [string]$originProject.id -and
            (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq
                [string]$originProject.id
    }) | Select-Object -First 1
    if ($null -eq $candidate -or $candidateMemberships.Count -ne 1) {
        throw "The reversible conversation membership changed during the read-only baseline refresh."
    }

    if ($ConfirmRoundTrip) {
        $moved = Invoke-ProjectMove -Conversation $candidate `
            -SourceProjectId ([string]$originProject.id) `
            -DestinationProject ([ref]$targetProject) `
            -WriteSelected ([ref]$forwardWriteSelected) -AllowFallbackDestination
        $forwardMoveVerified = $true
        if (-not (Restore-WebChatNativeConversation -Runtime $runtime `
                -ProviderId "chatgpt_web" -ConversationPath ([string]$moved.path) `
                -TimeoutSec ([Math]::Min($TimeoutSec, 120)))) {
            throw "The moved conversation did not reopen before the restore operation."
        }
        $restoreDestination = $originProject
        $restoredConversation = Invoke-ProjectMove -Conversation $moved `
            -SourceProjectId ([string]$targetProject.id) `
            -DestinationProject ([ref]$restoreDestination) `
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
            Request-MembershipRefresh -OriginProjectId ([string]$originProject.id) `
                -TargetProjectId ([string]$targetProject.id) `
                -ConversationPath ([string]$candidate.path)
            Start-Sleep -Seconds 2
            $recoveryNavigation = Get-Navigation
            $current = Get-CanonicalConversationMembership `
                -Navigation $recoveryNavigation -ConversationId ([string]$candidate.id)
            if ($null -eq $current) {
                $current = Get-LiveConversationMembership `
                    -ConversationId ([string]$candidate.id)
            }
            $currentProjectId = [string]$current.project_id
            if (
                $currentProjectId -eq [string]$targetProject.id -and
                -not $restoreWriteSelected
            ) {
                if (-not (Restore-WebChatNativeConversation -Runtime $runtime `
                        -ProviderId "chatgpt_web" -ConversationPath ([string]$current.path) `
                        -TimeoutSec ([Math]::Min($TimeoutSec, 120)))) {
                    throw "The moved conversation did not reopen for cleanup."
                }
                $cleanupDestination = $originProject
                $recovered = Invoke-ProjectMove -Conversation $current `
                    -SourceProjectId $currentProjectId `
                    -DestinationProject ([ref]$cleanupDestination) `
                    -WriteSelected ([ref]$cleanupWriteSelected)
                $restored = [string]$recovered.project_id -eq [string]$originProject.id
            } elseif (
                $currentProjectId -eq [string]$originProject.id
            ) {
                $restored = $true
            } elseif (
                $currentProjectId -eq [string]$targetProject.id -and
                $restoreWriteSelected
            ) {
                $recoveryDeadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
                do {
                    Request-MembershipRefresh -OriginProjectId ([string]$originProject.id) `
                        -TargetProjectId ([string]$targetProject.id) `
                        -ConversationPath ([string]$candidate.path)
                    Start-Sleep -Seconds 2
                    $recoveryNavigation = Get-Navigation
                    $current = Get-CanonicalConversationMembership `
                        -Navigation $recoveryNavigation `
                        -ConversationId ([string]$candidate.id)
                    if ([string]$current.project_id -eq [string]$originProject.id) {
                        $restored = $true
                        break
                    }
                } while ([DateTimeOffset]::UtcNow -lt $recoveryDeadline)
                if (-not $restored) { $recoveryUnknown = $true }
            } else {
                $recoveryUnknown = $true
            }
        } catch {
            if ($restoreWriteSelected -or $cleanupWriteSelected) {
                $recovered = Wait-ReadOnlyOriginalMembership `
                    -ConversationId ([string]$candidate.id) `
                    -ConversationPath ([string]$candidate.path) `
                    -OriginProjectId ([string]$originProject.id) `
                    -TargetProjectId ([string]$targetProject.id)
                $restored = $null -ne $recovered
                $recoveryUnknown = -not $restored
            } else {
                $recoveryUnknown = $true
            }
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

Write-Output (
    "CHATGPT_WEB_PROJECT_MOVE_RECOVERY=" +
    "forward_write_selected=$([bool]$forwardWriteSelected);" +
    "restore_write_selected=$([bool]$restoreWriteSelected);" +
    "cleanup_write_selected=$([bool]$cleanupWriteSelected);" +
    "restored=$([bool]$restored);" +
    "recovery_unknown=$([bool]$recoveryUnknown)"
)
if ($recoveryUnknown) {
    $recoveryDetail = "Project-move recovery is ambiguous; inspect the official project menu before retrying."
    if ($null -ne $primaryFailure) {
        throw ($primaryFailure.Exception.Message + " " + $recoveryDetail)
    }
    throw $recoveryDetail
}
if ($null -ne $primaryFailure) { throw $primaryFailure }
if ($null -ne $cleanupFailure) { throw $cleanupFailure }

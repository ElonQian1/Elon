#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [ValidateRange(30, 180)][int]$TimeoutSec = 120,
    [ValidateRange(0, 39)][int]$TargetProjectOffset = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion `
    -ExpectedAdapterVersion $ExpectedAdapterVersion

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

$originPath = ""
$candidate = $null
$originProject = $null
$targetProject = $null
$forwardDispatched = $false
$restoreDispatched = $false
$restored = $false
$recoveryUnknown = $false
$primaryFailure = $null

function Invoke-MainAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{}
    )

    return Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments
}

function Ensure-ProductionReady {
    $state = Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec
    $web = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec 15
    if (
        [int]$web.adapter_version -ne $ExpectedAdapterVersion -or
        $web.adapter_current -ne $true
    ) {
        throw "The installed ChatGPT adapter does not match the acceptance source."
    }
    if (
        [string]$state.active_surface -ne "social_ai" -or
        [string]$state.social_chat.web_chat_provider_id -ne "chatgpt_web" -or
        [string]$state.social_chat.web_chat_state -ne "ready" -or
        $state.social_chat.web_chat_composer_ready -ne $true
    ) {
        throw "The production ChatGPT surface is not ready."
    }
    return $state
}

function Invoke-ReadAction {
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
        $page = Invoke-ReadAction -Action "get_web_chat_navigation" -Arguments @{
            offset = $offset
            limit = 50
        }
        if ([string]$page.schema -ne "elon.web_chat.navigation.v1") {
            throw "Native ChatGPT navigation cache is unavailable."
        }
        @($page.conversations).ForEach({ $conversations.Add($_) })
        @($page.projects).ForEach({ $projects.Add($_) })
        $hasMore = $page.conversation_has_more -eq $true -or
            $page.project_has_more -eq $true
        $offset += 50
    } while ($hasMore -and $offset -le 500)
    return [pscustomobject]@{
        conversations = @($conversations)
        projects = @($projects)
    }
}

function Get-CanonicalProjectId {
    param([AllowEmptyString()][string]$Value)

    $trimmed = $Value.Trim()
    $production = [regex]::Match(
        $trimmed,
        '^(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]+)?$'
    )
    if ($production.Success) { return [string]$production.Groups[1].Value }
    if ($trimmed -match '^g-p-[A-Za-z0-9_-]{1,160}$') { return $trimmed }
    return ""
}

function Get-ConversationProjectIdFromPath {
    param([AllowEmptyString()][string]$Path)

    $match = [regex]::Match(
        $Path.Trim(),
        '^/g/(g-p-[A-Za-z0-9_-]{1,160})/c/[A-Za-z0-9_-]{1,160}$'
    )
    if (-not $match.Success) { return "" }
    return Get-CanonicalProjectId -Value ([string]$match.Groups[1].Value)
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

function Get-CanonicalMembership {
    param(
        [Parameter(Mandatory = $true)]$Navigation,
        [Parameter(Mandatory = $true)][string]$ConversationId
    )

    $matches = @($Navigation.conversations | Where-Object {
        [string]$_.id -eq $ConversationId -and
            (Get-CanonicalProjectId -Value ([string]$_.project_id)) -and
            (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq
                (Get-CanonicalProjectId -Value ([string]$_.project_id))
    })
    if ($matches.Count -ne 1) { return $null }
    return $matches[0]
}

function Request-ScopedRefresh {
    param(
        [Parameter(Mandatory = $true)][string]$ProjectId,
        [string]$ConversationPath = ""
    )

    $arguments = @{ project_id = $ProjectId }
    if ($ConversationPath.Trim()) {
        $arguments.conversation_path = $ConversationPath.Trim()
    }
    Invoke-ReadAction -Action "refresh_web_chat_conversations" `
        -Arguments $arguments | Out-Null
}

function Request-MembershipRefresh {
    param(
        [Parameter(Mandatory = $true)][string]$FirstProjectId,
        [Parameter(Mandatory = $true)][string]$SecondProjectId,
        [string]$ConversationPath = ""
    )

    Request-ScopedRefresh -ProjectId $FirstProjectId -ConversationPath $ConversationPath
    Start-Sleep -Milliseconds 800
    Request-ScopedRefresh -ProjectId $SecondProjectId -ConversationPath $ConversationPath
}

function Select-ReversibleSample {
    param([Parameter(Mandatory = $true)]$Navigation)

    $projectById = @{}
    foreach ($project in @($Navigation.projects)) {
        $id = Get-CanonicalProjectId -Value ([string]$project.id)
        if ($id -and -not $projectById.ContainsKey($id)) { $projectById[$id] = $project }
    }
    if ($projectById.Count -lt 2) {
        throw "At least two cached projects are required for reversible acceptance."
    }
    $eligible = @($Navigation.conversations | Where-Object {
        $id = [string]$_.id
        $projectId = Get-CanonicalProjectId -Value ([string]$_.project_id)
        $id -and $projectId -and $projectById.ContainsKey($projectId) -and
            (Get-ConversationIdentityFromPath -Path ([string]$_.path)) -eq $id -and
            (Get-ConversationProjectIdFromPath -Path ([string]$_.path)) -eq $projectId
    } | Group-Object { [string]$_.id } | Where-Object { $_.Count -eq 1 } |
        ForEach-Object { $_.Group[0] } | Sort-Object @{
            Expression = { if ($_.active -eq $true) { 1 } else { 0 } }
        })
    $conversation = $eligible | Select-Object -First 1
    if ($null -eq $conversation) {
        throw "No unambiguous project conversation is available for reversible acceptance."
    }
    $originId = Get-CanonicalProjectId -Value ([string]$conversation.project_id)
    $origin = $projectById[$originId]
    $target = @($projectById.GetEnumerator() | Where-Object {
        [string]$_.Key -ne $originId
    } | Sort-Object Key | Select-Object -Skip $TargetProjectOffset -First 1)
    if ($target.Count -ne 1) { throw "No alternate project is available." }
    return [pscustomobject]@{
        conversation = $conversation
        origin = $origin
        target = $target[0].Value
    }
}

function Wait-Membership {
    param(
        [Parameter(Mandatory = $true)][string]$ConversationId,
        [Parameter(Mandatory = $true)][string]$ConversationPath,
        [Parameter(Mandatory = $true)][string]$ExpectedProjectId,
        [Parameter(Mandatory = $true)][string]$OtherProjectId
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        Request-MembershipRefresh -FirstProjectId $ExpectedProjectId `
            -SecondProjectId $OtherProjectId -ConversationPath $ConversationPath
        Start-Sleep -Milliseconds 800
        $current = Get-CanonicalMembership -Navigation (Get-Navigation) `
            -ConversationId $ConversationId
        if (
            $null -ne $current -and
            (Get-CanonicalProjectId -Value ([string]$current.project_id)) -eq
                $ExpectedProjectId
        ) {
            return $current
        }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $null
}

function Invoke-PrivateMove {
    param(
        [Parameter(Mandatory = $true)]$Conversation,
        [Parameter(Mandatory = $true)][string]$ProjectId
    )

    $title = ([string]$Conversation.title).Trim()
    if ($title.Length -gt 160) { $title = $title.Substring(0, 160) }
    $dispatch = Invoke-MainAction -Action "chatgpt_move_conversation_to_project" `
        -Arguments @{
            conversation_path = [string]$Conversation.path
            conversation_title = $title
            project_id = $ProjectId
            user_confirmed = $true
        }
    $requestId = [string]$dispatch.command_receipt.request_id
    if (
        -not $requestId -or
        [string]$dispatch.command_receipt.expected_web_action -ne
            "move_conversation_to_project"
    ) {
        throw "The private project-move action did not return a valid receipt."
    }
    return Wait-ChatGptCommandReceipt -InvokeUiState {
        Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    } -RequestId $requestId -ExpectedAction "move_conversation_to_project" `
        -TimeoutSec $TimeoutSec -PollIntervalSec 1
}

try {
    $ready = Ensure-ProductionReady
    $originPath = [string]$ready.social_chat.web_chat_conversation_path
    $navigation = Get-Navigation
    $sample = Select-ReversibleSample -Navigation $navigation
    $candidate = $sample.conversation
    $originProject = $sample.origin
    $targetProject = $sample.target
    $conversationId = [string]$candidate.id
    $originProjectId = Get-CanonicalProjectId -Value ([string]$originProject.id)
    $targetProjectId = Get-CanonicalProjectId -Value ([string]$targetProject.id)

    Request-MembershipRefresh -FirstProjectId $originProjectId `
        -SecondProjectId $targetProjectId -ConversationPath ([string]$candidate.path)
    $baseline = Get-CanonicalMembership -Navigation (Get-Navigation) `
        -ConversationId $conversationId
    if (
        $null -eq $baseline -or
        (Get-CanonicalProjectId -Value ([string]$baseline.project_id)) -ne $originProjectId
    ) {
        throw "The reversible membership changed during baseline refresh."
    }
    $candidate = $baseline

    $forwardDispatched = $true
    Invoke-PrivateMove -Conversation $candidate -ProjectId $targetProjectId | Out-Null
    $moved = Wait-Membership -ConversationId $conversationId `
        -ConversationPath ([string]$candidate.path) -ExpectedProjectId $targetProjectId `
        -OtherProjectId $originProjectId
    if ($null -eq $moved) { throw "The forward project move did not reconcile." }

    $restoreDispatched = $true
    Invoke-PrivateMove -Conversation $moved -ProjectId $originProjectId | Out-Null
    $restoredConversation = Wait-Membership -ConversationId $conversationId `
        -ConversationPath ([string]$moved.path) -ExpectedProjectId $originProjectId `
        -OtherProjectId $targetProjectId
    $restored = $null -ne $restoredConversation
    if (-not $restored) { throw "The original project membership was not restored." }
} catch {
    $primaryFailure = $_
} finally {
    if ($forwardDispatched -and -not $restored -and $null -ne $candidate) {
        try {
            $conversationId = [string]$candidate.id
            $originProjectId = Get-CanonicalProjectId -Value ([string]$originProject.id)
            $targetProjectId = Get-CanonicalProjectId -Value ([string]$targetProject.id)
            $current = Wait-Membership -ConversationId $conversationId `
                -ConversationPath ([string]$candidate.path) `
                -ExpectedProjectId $originProjectId -OtherProjectId $targetProjectId
            if ($null -ne $current) {
                $restored = $true
            } elseif (-not $restoreDispatched) {
                $navigation = Get-Navigation
                $current = Get-CanonicalMembership -Navigation $navigation `
                    -ConversationId $conversationId
                if (
                    $null -ne $current -and
                    (Get-CanonicalProjectId -Value ([string]$current.project_id)) -eq
                        $targetProjectId
                ) {
                    $restoreDispatched = $true
                    Invoke-PrivateMove -Conversation $current `
                        -ProjectId $originProjectId | Out-Null
                    $restored = $null -ne (Wait-Membership `
                        -ConversationId $conversationId `
                        -ConversationPath ([string]$current.path) `
                        -ExpectedProjectId $originProjectId `
                        -OtherProjectId $targetProjectId)
                } else {
                    $recoveryUnknown = $true
                }
            } else {
                $recoveryUnknown = $true
            }
        } catch {
            $recoveryUnknown = $true
        }
    }
    if ($originPath) {
        try {
            Restore-WebChatNativeConversation -Runtime $runtime `
                -ProviderId "chatgpt_web" -ConversationPath $originPath `
                -TimeoutSec ([Math]::Min($TimeoutSec, 90)) | Out-Null
        } catch {}
    }
}

[ordered]@{
    schema = "elon.chatgpt_web.private_project_move_smoke.v1"
    passed = $null -eq $primaryFailure -and $restored -and -not $recoveryUnknown
    adapter_version = $ExpectedAdapterVersion
    forward_writes_invoked = if ($forwardDispatched) { 1 } else { 0 }
    restore_writes_invoked = if ($restoreDispatched) { 1 } else { 0 }
    original_membership_restored = $restored
    recovery_unknown = $recoveryUnknown
    private_content_emitted = $false
    cleared_cookies = $false
    cleared_app_data = $false
} | ConvertTo-Json -Depth 4

if ($recoveryUnknown) {
    throw "Private project-move recovery is ambiguous; do not repeat the write."
}
if ($null -ne $primaryFailure) { throw $primaryFailure }
Write-Output "CHATGPT_WEB_PRIVATE_PROJECT_MOVE_STATUS=passed"

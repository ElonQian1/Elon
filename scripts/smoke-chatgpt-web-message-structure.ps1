#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(20, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 50)][int]$MaxConversations = 20,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 61
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Wait-BridgeReady {
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "stable ChatGPT bridge" -Predicate {
            param($state)
            $state.surface -eq "chatgpt_web" -and
                $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true
        }
}

function Wait-ConversationPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $expectedPath = $Path
    $result = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "historical conversation structure" -Predicate {
            param($state)
            $state.surface -eq "chatgpt_web" -and
                $state.bridge_state -eq "ready" -and
                [string]$state.conversation.url -like "*$expectedPath*" -and
                [int]$state.conversation.message_count -gt 0
        }.GetNewClosure()
    Start-Sleep -Seconds $runtime.poll_interval_sec
    Wait-BridgeReady | Out-Null
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "stable historical conversation structure" -Predicate {
            param($state)
            $state.surface -eq "chatgpt_web" -and
                $state.bridge_state -eq "ready" -and
                [string]$state.conversation.url -like "*$Path*" -and
                [int]$state.conversation.message_count -gt 0
        }.GetNewClosure()
}

function Get-ContextWithParts {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        try {
            Wait-BridgeReady | Out-Null
            $context = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                -Action "chatgpt_get_context" -Arguments @{ message_offset = 0; message_limit = 50 }
        } catch {
            Start-Sleep -Seconds $runtime.poll_interval_sec
            continue
        }
        $messages = @($context.messages | Where-Object { $null -ne $_ })
        $parts = @(
            $messages |
                ForEach-Object { @($_.parts | Where-Object { $null -ne $_ }) }
        )
        if ($parts.Count -gt 0) {
            return [pscustomobject]@{ context = $context; messages = $messages; parts = $parts }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $null
}

function Get-ConversationPathFromUrl {
    param([string]$Url)

    if ([string]::IsNullOrWhiteSpace($Url)) { return "" }
    try {
        $uri = [Uri]$Url
        if ($uri.Host -notin @("chatgpt.com", "www.chatgpt.com")) { return "" }
        if ($uri.AbsolutePath -match '^/c/') { return $uri.AbsolutePath }
    } catch {
        return ""
    }
    return ""
}

function Open-ConversationPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        try {
            Wait-BridgeReady | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_open_conversation" `
                -Arguments @{ conversation_path = $Path } | Out-Null
            return Wait-ConversationPath -Path $Path
        } catch {
            if ($_.Exception.Message -notmatch 'bridge_not_ready') { throw }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out opening a ChatGPT conversation after bridge recovery."
}

function Wait-ConversationList {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        try {
            Wait-BridgeReady | Out-Null
            $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                -Action "chatgpt_get_conversations" -Arguments @{ offset = 0; limit = $MaxConversations }
            if (@($page.conversations).Count -gt 0) { return $page }
        } catch { }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "ChatGPT message structure verification is deferred without conversation history."
}

function Get-VisibleMessageSelectors {
    $remotePath = "/sdcard/elon-chatgpt-message-structure.xml"
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "uiautomator", "dump", $remotePath) -TimeoutSec 30 `
            -Label "dump native message UI" | Out-Null
        $xml = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "cat", $remotePath) -TimeoutSec 30 `
            -Label "read native message UI"
        return [pscustomobject]@{
            messages = @(
                [regex]::Matches($xml, 'content-desc="(chatgpt-message:[^"]+)"') |
                    ForEach-Object { $_.Groups[1].Value } |
                    Sort-Object -Unique
            )
            parts = @(
                [regex]::Matches($xml, 'content-desc="(chatgpt-message-part:[^"]+)"') |
                    ForEach-Object { $_.Groups[1].Value } |
                    Sort-Object -Unique
            )
        }
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remotePath) -TimeoutSec 10 `
            -Label "remove native message UI dump" | Out-Null
    }
}

function Wait-VisibleMessageSelectors {
    param([Parameter(Mandatory = $true)][string[]]$ExpectedPartSelectors)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $visible = Get-VisibleMessageSelectors
        $matched = @($visible.parts | Where-Object { $_ -in $ExpectedPartSelectors })
        if ($visible.messages.Count -gt 0 -and $matched.Count -gt 0) {
            return [pscustomobject]@{ visible = $visible; matched = $matched }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $null
}

function Restore-Origin {
    param(
        [Parameter(Mandatory = $true)]$Origin,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$OriginPath
    )

    $current = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    if ([string]$current.surface -ne "chatgpt_web") {
        throw "ChatGPT surface left the foreground before origin restoration."
    }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "web" } | Out-Null
    if ($OriginPath) {
        Open-ConversationPath -Path $OriginPath | Out-Null
    } else {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_new_conversation" | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "restored blank conversation" -Predicate {
                param($state)
                $state.surface -eq "chatgpt_web" -and
                    $state.bridge_state -eq "ready" -and
                    [int]$state.conversation.message_count -eq 0 -and
                    $state.streaming -eq $false
            } | Out-Null
    }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = [string]$Origin.view_mode } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "restored ChatGPT view mode" -Predicate {
            param($state)
            [string]$state.view_mode -eq [string]$Origin.view_mode
        } | Out-Null
}

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
    -EnsureMainActivity | Out-Null
$origin = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
    -Description "authenticated ChatGPT message surface" -Predicate {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true
    }
Assert-ChatGptWebSmokeAdapterVersion -State $origin `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
if ([int]$origin.input.text_length -gt 0) {
    throw "ChatGPT message structure verification is deferred while a draft is present."
}
$originPath = Get-ConversationPathFromUrl -Url ([string]$origin.conversation.url)
$restoreRequired = $false
$result = $null
try {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "web" } | Out-Null
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_list_conversations" | Out-Null
    $page = Wait-ConversationList
    $candidates = @(
        $page.conversations |
            Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.path) } |
            Select-Object -First $MaxConversations
    )
    if ($candidates.Count -eq 0) {
        throw "ChatGPT message structure verification is deferred without conversation history."
    }

    $sample = $null
    $inspected = 0
    foreach ($candidate in $candidates) {
        $path = [string]$candidate.path
        $restoreRequired = $true
        Open-ConversationPath -Path $path | Out-Null
        $inspected += 1
        $sample = Get-ContextWithParts
        if ($null -ne $sample) { break }
    }
    if ($null -eq $sample) {
        throw "ChatGPT structured message sample is unavailable; verification is deferred."
    }

    $expectedSelectors = @(
        $sample.parts |
            ForEach-Object { [string]$_.native_adb_content_description } |
            Where-Object { $_ -match '^chatgpt-message-part:' } |
            Sort-Object -Unique
    )
    if ($expectedSelectors.Count -ne $sample.parts.Count) {
        throw "Structured message parts do not all expose stable native selectors."
    }
    $partTypes = @(
        $sample.parts |
            ForEach-Object { [string]$_.type } |
            Where-Object { $_ } |
            Sort-Object -Unique
    )
    $sampleMessage = $sample.messages |
        Where-Object { @($_.parts | Where-Object { $null -ne $_ }).Count -gt 0 } |
        Select-Object -First 1
    if ($null -eq $sampleMessage) {
        throw "Structured message sample lost its owning message."
    }
    $targetSelector = [string]$sampleMessage.parts[0].native_adb_content_description

    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "native" } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "native structured message surface" -Predicate {
            param($state)
            $state.view_mode -eq "native" -and $state.bridge_state -eq "ready"
        } | Out-Null
    $reveal = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_reveal_message" `
        -Arguments @{ message_id = [string]$sampleMessage.id; part_index = 0 }
    if ($reveal.control_ok -ne $true) {
        throw "Native structured message reveal action failed."
    }
    $visibleResult = Wait-VisibleMessageSelectors -ExpectedPartSelectors @($targetSelector)
    if ($null -eq $visibleResult) {
        throw "Native structured message selectors are not visible in the Android UI tree."
    }
    $visible = $visibleResult.visible
    $matchedSelectors = @($visibleResult.matched)

    $result = [ordered]@{
        schema = "elon.chatgpt_web.message_structure_smoke.v1"
        passed = $true
        device_serial = $DeviceSerial
        inspected_conversation_count = $inspected
        message_count = $sample.messages.Count
        message_part_count = $sample.parts.Count
        message_part_types = $partTypes
        context_selector_count = $expectedSelectors.Count
        visible_message_selector_count = $visible.messages.Count
        visible_part_selector_count = $visible.parts.Count
        matched_part_selector_count = $matchedSelectors.Count
        reveal_action_succeeded = $true
        original_conversation_restored = $true
        original_view_mode_restored = $true
        sent_messages = 0
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
    }
} finally {
    if ($restoreRequired) {
        Restore-Origin -Origin $origin -OriginPath $originPath
    }
}

$result | ConvertTo-Json -Depth 8
Write-Output "CHATGPT_WEB_MESSAGE_STRUCTURE_SMOKE_STATUS=passed"

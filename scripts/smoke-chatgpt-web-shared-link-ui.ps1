#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe',
    [switch]$ResumeFixture
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-shared-link-fixture.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{schema='elon.chatgpt.shared_link_ui.v1'; passed=$false; stage='prepare'
    original_restored=$false; awake_restored=$false; content_exported=$false; sent_messages=0
    created_links=0; native_revoke_confirmed=$false; cleanup_confirmed=$false}
$origin = $null; $probe = ''; $url = ''; $shareId = ''; $revokeDispatched = $false; $creationAttempted = $false
$beforeIds = @(); $opened = $false
$checkpoint = Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/shared-link-fixture.json'

function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
}
function Act([string]$Action, [hashtable]$Arguments = @{}) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
}
function Receipt([hashtable]$Arguments) {
    $sent = Act 'chatgpt_share_conversation' $Arguments
    try {
        $result = Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id `
            -ExpectedAction share_conversation -TimeoutSec 25 -PollIntervalSec 1
    } catch {
        $failed = @((Web).command_requests | Where-Object request_id -ceq $sent.command_receipt.request_id) | Select-Object -Last 1
        if ($failed.status -in @('failed','timed_out') -and $failed.result.detail -cmatch '^share_[a-z0-9_]+$') {
            throw ([string]$failed.result.detail)
        }
        throw
    }
    return $result.receipt.result.detail
}
function Ui([string]$Step) {
    Write-Host ('SHARE_UI_ACTION=' + $Step)
    $parameters = @{url_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($url))}
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass SharedLinkUiAcceptance -Step $Step `
        -Parameters $parameters -ResultPrefix SHARED_LINK_UI_RESULT
}
function Menu([string]$Step) {
    Write-Host ('SHARE_MENU_ACTION=' + $Step)
    $raw = @(& (Join-Path $PSScriptRoot 'invoke-conversation-ui-acceptance.ps1') `
        -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial -Step $Step)
    $raw[-1] | ConvertFrom-Json
}
function Open-AccountList {
    $ids = @((Web).command_requests | ForEach-Object request_id)
    Act 'open_chat_side_menu' | Out-Null
    $script:opened = $true
    Menu 'conversation_actions' | Out-Null
    Menu 'share' | Out-Null
    Menu 'account_shares' | Out-Null
    Menu 'share_list' | Out-Null
    $receipt = @((Web).command_requests | Where-Object {
        $_.request_id -notin $ids -and $_.expected_web_action -eq 'share_conversation' -and $_.status -eq 'succeeded'
    }) | Select-Object -Last 1
    if (-not $receipt) { throw 'native_account_list_unconfirmed' }
    $page = $receipt.result.detail | ConvertFrom-Json
    if ($page.schema -ne 'elon.account_shares.v1' -or -not $page.complete -or $page.offset -ne 0 -or
        $page.items[0].id -cne $shareId -or $page.items[0].path -cne $probe) { throw 'new_share_not_first_in_confirmed_list' }
    return $page
}
function Read-All {
    $index = (Receipt @{operation='list_account'}) | ConvertFrom-Json
    if ($index.schema -ne 'elon.account_shares.v1' -or -not $index.complete -or $null -ne $index.nextOffset) {
        throw 'account_snapshot_not_complete'
    }
    return $index
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin = Get-ChatGptWebNativeChatState -Runtime $runtime
    $web = Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $web.authenticated -or -not $web.adapter_current -or $origin.input.text -or $web.streaming -or
        $web.dictation_active -or [int]$web.input.official_draft_length -gt 0) { throw 'existing_work_or_unready_surface' }
    if ($origin.social_chat.web_chat_conversation_path -notmatch '^(?:/g/[A-Za-z0-9_-]{1,160})?/c/[a-fA-F0-9-]{36}$') { throw 'existing_origin_required' }
    $report.adapter = $web.adapter_version
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $beforeIds = @((Read-All).items | ForEach-Object id)
    $report.stage = 'synthetic_conversation'
    Write-Output 'SHARE_UI_STAGE=synthetic_conversation'
    if ($ResumeFixture) {
        $saved = Get-Content -LiteralPath $checkpoint -Raw | ConvertFrom-Json
        if ($saved.schema -ne 'elon.synthetic_shared_link_fixture.v1' -or $saved.hardware -cne $ExpectedHardwareSerial -or
            $saved.path -notmatch '^/c/[a-f0-9-]{36}$' -or $saved.marker -cnotmatch '^ELON-SHARE-UI-[a-f0-9]{32}$') { throw 'invalid_fixture_checkpoint' }
        $marker = [string]$saved.marker; $probe = [string]$saved.path
        # ResumeFixture needs the live identity page, not only the cache-first native route.
        $navigation = Act 'chatgpt_open_conversation' @{conversation_path=$probe}
        Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $navigation.command_receipt.request_id `
            -ExpectedAction open_conversation -TimeoutSec 40 -PollIntervalSec 1 | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 40 -Description 'live sharing fixture navigation' -Predicate {
            param($state)
            $state.bridge_state -eq 'ready' -and $state.adapter_current -and
                ([uri]$state.conversation.url).AbsolutePath -ceq $probe
        }.GetNewClosure() | Out-Null
    } else {
        Act 'start_new_web_chat_conversation' | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec 40 -Description 'blank native sharing fixture' -Predicate {
            param($state)
            $state.social_chat.web_chat_state -eq 'ready' -and $state.social_chat.web_chat_composer_ready -eq $true -and
                [int]$state.social_chat.message_count -eq 0
        } | Out-Null
        $marker = 'ELON-SHARE-UI-' + [Guid]::NewGuid().ToString('N')
        Act 'set_input_text' @{text=('Reply only with this exact test marker: ' + $marker)} | Out-Null
        Act 'send_input' | Out-Null
        $report.sent_messages = 1
        $anchored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 40 -Description 'sharing fixture identity' -Predicate {
            param($state)
            $state.conversation.url -match '/c/[a-f0-9-]{36}$' -and
                @($state.conversation.messages | Where-Object { $_.role -eq 'user' -and ([string]$_.content).Contains($marker) }).Count -eq 1
        }.GetNewClosure()
        $probe = ([uri]$anchored.conversation.url).AbsolutePath
        New-Item -ItemType Directory -Path (Split-Path -Parent $checkpoint) -Force | Out-Null
        [IO.File]::WriteAllText($checkpoint, (@{schema='elon.synthetic_shared_link_fixture.v1';hardware=$ExpectedHardwareSerial;
            path=$probe;marker=$marker} | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    }
    $prompt = 'Reply only with this exact test marker: ' + $marker
    $reply = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 90 -Description 'completed synthetic sharing reply' -Predicate {
        param($state)
        $messages = @($state.conversation.messages)
        $state.bridge_state -eq 'ready' -and -not $state.streaming -and $messages.Count -ge 2 -and
            @($messages | Where-Object { $_.role -eq 'assistant' -and $_.state -eq 'completed' }).Count -ge 1
    }
    $probe = ([uri]$reply.conversation.url).AbsolutePath
    if ($probe -notmatch '^/c/[a-f0-9-]{36}$' -or $probe -ceq $origin.social_chat.web_chat_conversation_path) { throw 'fixture_scope_mismatch' }
    $context = Act 'chatgpt_get_context' @{message_offset=0;message_limit=10}
    $report.fixture_evidence = @{count=@($context.messages).Count; same_url=($context.conversation_url -ceq $reply.conversation.url)
        prompt_matches=($context.messages[0].content -ceq $prompt); reply_matches=(([string]$context.messages[1].content).Trim() -ceq $marker)
        rows=@($context.messages | ForEach-Object { @{role=$_.role;state=$_.state;chars=([string]$_.content).Length;
            truncated=$_.content_truncated;parts=@($_.parts | ForEach-Object type)} })}
    if (-not (Test-ChatGptSharedLinkFixture -Context $context -ExpectedUrl $reply.conversation.url `
        -Prompt $prompt -Marker $marker)) { throw 'fixture_content_not_safe_to_publish' }
    $report.stage = 'verify_unshared_fixture'
    Write-Output 'SHARE_UI_STAGE=verify_unshared_fixture'
    $initial = (Receipt @{operation='list';conversation_path=$probe}) | ConvertFrom-Json
    if (-not $initial.complete -or @($initial.items).Count -ne 0) { throw 'new_fixture_already_shared' }
    $report.stage = 'create_one_link'
    Write-Output 'SHARE_UI_STAGE=create_one_link'
    $creationAttempted = $true
    $created = Receipt @{conversation_path=$probe;user_confirmed=$true}
    if ($created -cnotmatch '^share_link_ready:(https://chatgpt\.com/share/([a-f0-9-]{36}))$') { throw 'share_creation_unconfirmed' }
    $url = $Matches[1]; $shareId = $Matches[2]; $report.created_links = 1
    if ($shareId -in $beforeIds) { throw 'created_link_was_preexisting' }
    if (-not (Restore-WebChatNativeConversation -Runtime $runtime -ProviderId chatgpt_web `
        -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 40)) { throw 'return_for_account_list_failed' }
    $report.stage = 'native_copy'
    Write-Output 'SHARE_UI_STAGE=native_copy'
    Open-AccountList | Out-Null
    Ui 'select_first' | Out-Null
    Ui 'copy' | Out-Null
    Act 'close_chat_side_menu' | Out-Null
    $paste = Ui 'paste_check'
    $report.clipboard_exact_match = $paste.clipboard_exact_match
    $report.test_draft_cleared = $paste.test_draft_cleared
    $report.stage = 'native_revoke'
    Write-Output 'SHARE_UI_STAGE=native_revoke'
    Open-AccountList | Out-Null
    Ui 'select_first' | Out-Null
    $ids = @((Web).command_requests | ForEach-Object request_id)
    $revokeDispatched = $true
    Ui 'revoke' | Out-Null
    $afterRevoke = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 30 -Description 'native selected share removal' -Predicate {
        param($state)
        @($state.command_requests | Where-Object { $_.request_id -notin $ids -and $_.status -eq 'succeeded' -and
            $_.expected_web_action -eq 'share_conversation' -and $_.result.detail -eq 'share_link_revoked' }).Count -eq 1
    }.GetNewClosure()
    $report.native_revoke_confirmed = $true
    $afterIds = @((Read-All).items | ForEach-Object id)
    $report.cleanup_confirmed = $shareId -notin $afterIds
    $report.other_links_unchanged = @($beforeIds | Where-Object { $_ -notin $afterIds }).Count -eq 0 -and
        @($afterIds | Where-Object { $_ -notin $beforeIds }).Count -eq 0
    if (-not $report.cleanup_confirmed -or -not $report.other_links_unchanged) { throw 'share_readback_mismatch' }
    Menu 'close_shares' | Out-Null
    Act 'close_chat_side_menu' | Out-Null
    $opened = $false
    $report.stage = 'complete'
    $report.passed = $true
} catch {
    $code = [string]$_.Exception.Message
    $report.error = if ($code -cmatch '^[a-z_]+$') { $code } else { 'acceptance_failed' }
    $safe = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $code -MaxLength 180
    Write-Output ('SHARE_UI_FAILURE=' + $safe)
} finally {
    try {
        if ($creationAttempted -and -not $shareId) {
            $reconcile = (Receipt @{operation='list';conversation_path=$probe}) | ConvertFrom-Json
            if ($reconcile.complete -and @($reconcile.items).Count -eq 1) {
                $shareId = [string]$reconcile.items[0].id
                $report.created_links = 1
            } elseif ($reconcile.complete -and @($reconcile.items).Count -eq 0) { $report.cleanup_confirmed = $true }
            else { $report.cleanup_pending = $true }
        }
        if ($shareId -and $shareId -notin $beforeIds -and -not $report.cleanup_confirmed) {
            $fresh = Read-All
            $selected = @($fresh.items | Where-Object { $_.id -ceq $shareId -and $_.path -ceq $probe })
            if ($selected.Count -eq 1 -and -not $revokeDispatched) {
                $revokeDispatched = $true
                Receipt @{operation='revoke';conversation_path=$probe;share_id=$shareId;selection_ticket=$fresh.ticket;user_confirmed=$true} | Out-Null
                $fresh = Read-All
            }
            $report.cleanup_confirmed = $shareId -notin @($fresh.items | ForEach-Object id)
        }
    } catch { $report.cleanup_pending = $true }
    try {
        if ($opened) {
            if ((Menu 'inspect').account_share_page) { Menu 'close_shares' | Out-Null }
            elseif ((Menu 'inspect').share_menu) { Menu 'back' | Out-Null }
            elseif ($url) {
                $selected = Ui 'inspect'
                if ($selected.selected_dialog -or $selected.revoke_dialog) { Menu 'back' | Out-Null }
            }
            Act 'close_chat_side_menu' | Out-Null
        }
    } catch { $report.menu_restoration_failed = $true }
    try {
        if ($null -ne $origin) {
            $main = Get-ChatGptWebNativeChatState -Runtime $runtime
            if ($main.input.text -ceq $url -or $main.input.text -ceq $prompt) { Act 'set_input_text' @{text=''} | Out-Null }
            $report.original_restored = Restore-WebChatNativeConversation -Runtime $runtime -ProviderId chatgpt_web `
                -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 40
            $main = Get-ChatGptWebNativeChatState -Runtime $runtime
            $report.original_restored = $report.original_restored -and $main.input.text -ceq $origin.input.text
        }
    } catch { $report.restoration_failed = $true }
    $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime
    $report.passed = $report.passed -and $report.cleanup_confirmed -and $report.original_restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 5 -Compress
}
if (-not $report.passed) { exit 1 }

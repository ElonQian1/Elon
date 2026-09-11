#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateRange(1, 12)][int]$Limit = 6,
    [ValidateRange(0, 49)][int]$CandidateOffset = 0,
    [switch]$NativeMenu,
    [switch]$VerifyRefreshInPlace,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
if ($VerifyRefreshInPlace -and -not $NativeMenu) { throw 'refresh_check_requires_native_menu' }
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{ schema = 'elon.chatgpt.file_reference_inventory.v1'; passed = $false
    inventory_completed = $false
    restored = $false; awake_restored = $false; sent_messages = 0; downloaded_files = 0
    content_exported = $false; cases = @() }
$origin = $null; $navigated = $false; $probeStarted = $false
$fixtures = @('elon-chatgpt-attachment-fixture-v1.txt', 'elon-chatgpt-media-fixture-v1.png',
    'elon-chatgpt-media-fixture-v1.pdf')
function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
}
function Ui([string]$Step) {
    $report.ui_step = $Step
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance `
        -Step $Step -ResultPrefix CONVERSATION_UI_RESULT
}
function Act([string]$Action, [hashtable]$Arguments, [string]$Expected) {
    $sent = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
    try {
        return Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id `
            -ExpectedAction $Expected -TimeoutSec 20 -PollIntervalSec 1
    } catch {
        $receipt = @((Web).command_requests | Where-Object request_id -EQ $sent.command_receipt.request_id) | Select-Object -Last 1
        if ($receipt.result.detail -cmatch '^[a-z_]{1,80}$') { throw $receipt.result.detail }
        throw 'command_unconfirmed'
    }
}
function Open([string]$Path) {
    Act 'chatgpt_open_conversation' @{ conversation_path = $Path } 'open_conversation' | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $state = Get-ChatGptWebNativeChatState -Runtime $runtime
        $web = Web
        if ($state.social_chat.web_chat_conversation_path -eq $Path -and
            ([uri]$web.conversation.url).AbsolutePath -eq $Path -and $web.adapter_current -and
            -not $web.streaming -and -not $web.dictation_active) { return $state }
        Start-Sleep -Milliseconds 400
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'conversation_navigation_unconfirmed'
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready) { throw 'device_locked' }
    $origin = Get-ChatGptWebNativeChatState -Runtime $runtime
    $before = Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or -not $before.adapter_current -or
        -not $origin.social_chat.web_chat_conversation_path) { throw 'surface_not_ready' }
    if ($origin.input.text -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -gt 0 -or $before.file_download.can_cancel -or
        [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0) { throw 'existing_work_in_progress' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $report.adapter = $before.adapter_version
    $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action 'chatgpt_get_conversations' `
        -Arguments @{ offset = 0; limit = 50 }
    if ($page.stale) { throw 'directory_stale' }
    $report.directory_returned = @($page.conversations).Count
    $report.directory_has_more = [bool]$page.has_more
    $candidates = @($page.conversations | Where-Object {
        $_.title -match '(?i)fixture|file|attachment|test|\u5a92\u4f53|\u9644\u4ef6|\u6587\u4ef6'
    } | Select-Object -Skip $CandidateOffset -First $Limit)
    $report.candidate_offset = $CandidateOffset
    $report.candidates = $candidates.Count
    foreach ($candidate in $candidates) {
        $case = [ordered]@{ ordinal = $report.cases.Count; index_read = $false }
        $menuOpened = $false
        $report.cases += $case
        try {
            $navigated = $true
            $report.stage = 'navigate'
            Open $candidate.path | Out-Null
            Act 'chatgpt_private_protocol_probe' @{ mode = 'start' } 'private_protocol_probe' | Out-Null
            $probeStarted = $true
            $report.stage = 'file_index'
            if ($NativeMenu) {
                $menuOpened = $true
                Ui 'header' | Out-Null
                Ui 'current_settings' | Out-Null
                Ui 'files' | Out-Null
                if ($VerifyRefreshInPlace) {
                    Ui 'files_wait' | Out-Null
                    $priorRequestIds = @((Web).command_requests.request_id)
                    $refresh = Ui 'files_refresh_stable'
                    $case.refresh = $refresh.refresh
                } else { Ui 'files_refresh' | Out-Null }
                $menu = Ui 'files_wait'
                if ($VerifyRefreshInPlace) {
                    $reads = @((Web).command_requests | Where-Object {
                        $_.request_id -notin $priorRequestIds -and $_.expected_web_action -eq 'list_conversation_files'
                    })
                    $case.refresh_read_commands = $reads.Count
                    if (-not $case.refresh.same_window -or $reads.Count -eq 0 -or
                        @($reads | Where-Object { $_.status -ne 'succeeded' -or $_.result.ok -ne $true }).Count -gt 0) {
                        throw 'refresh_receipt_unconfirmed'
                    }
                    if ($case.refresh.loading_samples -eq 3 -and $reads.Count -ne 1) { throw 'duplicate_pending_read' }
                }
                $case.native_menu = @{ visible = $menu.file_index_visible
                    first_row = $menu.file_index_first_row; empty = $menu.file_index_empty }
            } else {
                Act 'chatgpt_list_conversation_files' @{ conversation_path = $candidate.path } 'list_conversation_files' | Out-Null
            }
            $index = (Web).conversation_files
            if (-not $index -or $index.stale -or $index.conversation_path -ne $candidate.path) { throw 'file_index_unconfirmed' }
            $files = @($index.files)
            $assistant = @($files | Where-Object role -eq assistant)
            $case.index_read = $true
            $case.files = $files.Count
            $case.truncated = [bool]$index.truncated
            $case.assistant_files = $assistant.Count
            $case.assistant_downloadable = @($assistant | Where-Object download_handle).Count
            $case.fixture_files = @($files | Where-Object name -CIn $fixtures).Count
            $case.assistant_fixture_files = @($assistant | Where-Object name -CIn $fixtures).Count
        } catch {
            $case.error = if ($_.Exception.Message -match '^[a-z_]+$') { $_.Exception.Message }
                elseif ($_.Exception.Message -match '^Semantic UI acceptance failed: ([a-z_]{1,80})$') { $Matches[1] }
                else { 'case_failed' }
            if ($NativeMenu) { $case.ui_step = $report.ui_step }
        } finally {
            if ($menuOpened) {
                try { Ui 'back' | Out-Null } catch { $case.menu_restore_failed = $true; $case.index_read = $false }
            }
            if ($probeStarted) {
                try {
                    $stopped = Act 'chatgpt_private_protocol_probe' @{ mode = 'stop' } 'private_protocol_probe'
                    $probe = $stopped.receipt.result.detail | ConvertFrom-Json
                    $case.protocol = @($probe.records | Select-Object method, path, status)
                    $case.protocol_dropped = $probe.dropped
                } catch { $case.protocol_unavailable = $true }
                $probeStarted = $false
            }
        }
    }
    $report.inventory_completed = $true
    $report.passed = $candidates.Count -gt 0 -and @($report.cases | Where-Object { -not $_.index_read }).Count -eq 0
} catch {
    $report.error = if ($_.Exception.Message -match '^[a-z_]+$') { $_.Exception.Message } else { 'inventory_failed' }
} finally {
    if ($probeStarted) {
        try {
            $stopped = Act 'chatgpt_private_protocol_probe' @{ mode = 'stop' } 'private_protocol_probe'
            $probe = $stopped.receipt.result.detail | ConvertFrom-Json
            $report.protocol = @($probe.records | Select-Object method, path, status)
            $report.protocol_dropped = $probe.dropped
        } catch { $report.protocol_unavailable = $true }
    }
    try {
        if ($navigated -and $origin) { Open $origin.social_chat.web_chat_conversation_path | Out-Null }
        if ($origin) {
            $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
            do {
                $after = Get-ChatGptWebNativeChatState -Runtime $runtime
                $web = Web
                $report.restored = $after.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                    @($after.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                    $after.input.text -eq $origin.input.text -and [int]$web.input.official_draft_length -eq 0
                if ($report.restored) { break }
                Start-Sleep -Milliseconds 400
            } while ([DateTimeOffset]::UtcNow -lt $deadline)
        }
    } catch { $report.restored = $false }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime } catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 6 -Compress
}
if (-not $report.passed) { exit 1 }

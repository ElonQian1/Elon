#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{ schema = 'elon.chatgpt.library_navigation.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; private_content_emitted = $false }
$origin = $null; $browserOpened = $false; $inFolder = $false

function Read-Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
}

function Act([string]$Action) {
    $raw = @(& $runtime.invoke_mcp -Adb $runtime.adb -DeviceSerial $DeviceSerial -Tool ui_control `
        -Arguments (@{ action = $Action } | ConvertTo-Json -Compress) -NoBootstrap `
        -HealthTimeoutSec 5 -RequestTimeoutSec 30 -AdbTimeoutSec 8) | Select-Object -Last 1
    if ($raw.result.isError -or $raw.result.structuredContent.control_ok -ne $true) { throw 'action_unconfirmed' }
}

function Ui([string]$Step, [string]$Handle = '') {
    $raw = @(& (Join-Path $PSScriptRoot 'invoke-library-ui-acceptance.ps1') `
        -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial -Step $Step -Handle $Handle)
    return $raw[-1] | ConvertFrom-Json
}

function Navigate([string]$Step, [string]$Handle = '') {
    $report.stage = $Step
    $before = Read-Web
    $ids = @($before.command_requests | ForEach-Object request_id)
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $visible = Ui $Step $Handle
    if ($visible.library_visible -ne $true) { throw 'library_not_visible' }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $state = Read-Web
        $receipt = @($state.command_requests | Where-Object {
            $_.expected_web_action -eq 'list_library_files' -and $_.request_id -notin $ids
        }) | Select-Object -Last 1
        if ($receipt.status -in @('failed', 'timed_out')) {
            $report.read_result = $receipt.result.detail
            throw 'library_read_unconfirmed'
        }
        if ($receipt.status -eq 'succeeded' -and $state.library_files.request_id -eq $receipt.request_id) {
            $page = $state.library_files
            if ($page.stale -eq $true) { throw 'stale_page_after_success' }
            Write-Output -InputObject ([pscustomobject]@{ page = $page; elapsed_ms = $timer.ElapsedMilliseconds }) -NoEnumerate
            return
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'library_receipt_timeout'
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin = Get-ChatGptWebNativeChatState -Runtime $runtime
    $web = Read-Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $origin.social_chat.interaction_mode -ne 'chat' -or $web.authenticated -ne $true) { throw 'surface_not_ready' }
    if ($web.streaming -or $web.dictation_active -or $origin.input.text -or
        [int]$web.input.official_draft_length -gt 0) { throw 'existing_work_in_progress' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    Act 'open_chat_side_menu'
    $browserOpened = $true
    $first = Navigate 'browse'
    if ($first.page.directory_handle -or $first.page.query) { throw 'root_scope_mismatch' }
    $report.root_count = @($first.page.items).Count
    $report.root_has_more = $first.page.has_more
    if ($first.page.has_more -ne $true) { throw 'pagination_fixture_unavailable' }
    $second = Navigate 'more'
    $initial = @($first.page.items | ForEach-Object handle)
    $all = @($second.page.items | ForEach-Object handle)
    $report.next_count = $all.Count
    $report.next_action_ms = $second.elapsed_ms
    $report.earlier_rows_retained = @($initial | Where-Object { $_ -notin $all }).Count -eq 0
    $report.unique_handles = @($all | Select-Object -Unique).Count -eq $all.Count
    if (-not $report.earlier_rows_retained -or -not $report.unique_handles -or $all.Count -le $initial.Count) {
        throw 'pagination_not_confirmed'
    }
    $folder = $second.page.items | Where-Object kind -eq 'directory' | Select-Object -First 1
    if ($null -eq $folder) { throw 'folder_fixture_unavailable' }
    $inFolder = $true
    $child = Navigate 'file' $folder.handle
    $report.folder_count = @($child.page.items).Count
    if ($child.page.directory_handle -ne $folder.handle -or @($child.page.breadcrumbs).Count -ne 1) {
        throw 'folder_scope_mismatch'
    }
    $back = Navigate 'back'
    $inFolder = $false
    if ($back.page.directory_handle -or @($back.page.breadcrumbs).Count -ne 0 -or
        @($back.page.items).Count -ne $all.Count) { throw 'root_restore_mismatch' }
    $report.folder_back_passed = $true
    $report.passed = $true
    $report.stage = 'complete'
} catch {
    $message = [string]$_.Exception.Message
    $report.error = if ($message -match '^[a-z_]+$') { $message } else { 'acceptance_failed' }
} finally {
    try {
        if ($browserOpened) {
            $visible = Ui 'inspect'
            if ($visible.library_visible) {
                if ($inFolder) { Ui 'back' | Out-Null }
                Ui 'back' | Out-Null
            }
            Act 'close_chat_side_menu'
        }
        if ($null -ne $origin) {
            $after = Get-ChatGptWebNativeChatState -Runtime $runtime
            $web = Read-Web
            $report.restored = $after.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                @($after.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                $after.input.text -eq $origin.input.text -and [int]$web.input.official_draft_length -eq 0 -and
                -not $web.streaming -and -not $web.dictation_active
        }
    } catch { $report.restored = $false }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime }
    catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 4 -Compress
}
if (-not $report.passed -or -not $report.restored -or -not $report.awake_restored) { exit 1 }

#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory)][switch]$AllowExistingPhoneOnlyDownload,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{ schema = 'elon.chatgpt.library_folder_download.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; content_exported = $false
    source_mutated = $false; sent_messages = 0; protocol = @() }
$origin = $null; $browserOpened = $false; $probeStarted = $false; $directory = ''

function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
}
function Act([string]$Action, [hashtable]$Arguments = @{}, [string]$Expected = '') {
    $sent = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
    if ($Expected) {
        return Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id `
            -ExpectedAction $Expected -TimeoutSec 25 -PollIntervalSec 1
    }
    return $sent
}
function Ui([string]$Step, [hashtable]$Parameters = @{}) {
    return Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass LibraryUiAcceptance `
        -Step $Step -Parameters $Parameters -ResultPrefix LIBRARY_UI_RESULT
}
function Navigation([string]$Step, [hashtable]$Parameters = @{}) {
    $ids = @((Web).command_requests | ForEach-Object request_id)
    Ui $Step $Parameters | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        $web = Web
        $receipt = @($web.command_requests | Where-Object {
            $_.request_id -notin $ids -and $_.expected_web_action -eq 'list_library_files'
        }) | Select-Object -Last 1
        if ($receipt.status -in @('failed', 'timed_out')) { throw 'library_read_failed' }
        if ($receipt.status -eq 'succeeded' -and $web.library_files.request_id -eq $receipt.request_id -and
            $web.library_files.stale -ne $true) { return $web.library_files }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'library_read_timeout'
}
function Probe([string]$Mode) {
    return Act 'chatgpt_private_protocol_probe' @{ mode = $Mode } 'private_protocol_probe'
}

try {
    if (-not $AllowExistingPhoneOnlyDownload) { throw 'explicit_download_authorization_required' }
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready) { throw 'device_locked' }
    $origin = Get-ChatGptWebNativeChatState -Runtime $runtime
    $before = Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or -not $before.adapter_current) { throw 'surface_not_ready' }
    if ($origin.input.text -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -gt 0 -or $before.file_download.can_cancel -or
        [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0) { throw 'existing_work_in_progress' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $report.adapter = $before.adapter_version
    $report.stage = 'native_folder'
    Act 'open_chat_side_menu' | Out-Null
    $browserOpened = $true
    $page = Navigation 'browse'
    if ($page.directory_handle -or $page.query) { $page = Navigation 'clear_query' }
    $folder = @($page.items | Where-Object kind -eq directory)
    if ($folder.Count -ne 1) { throw 'single_folder_fixture_unavailable' }
    $directory = $folder[0].handle
    $child = Navigation 'file' @{ handle = $directory }
    if ($child.directory_handle -ne $directory -or $child.stale -or $child.partial) { throw 'folder_scope_mismatch' }
    $file = $child.items | Where-Object {
        $_.kind -eq 'file' -and $_.media_type -eq 'image/png' -and $_.download_handle -and
        $_.size_bytes -gt 0 -and $_.size_bytes -le 524288 -and $_.name -cmatch '^[A-Za-z0-9_. -]{1,120}\.png$'
    } | Sort-Object size_bytes | Select-Object -First 1
    if (-not $file) { throw 'bounded_png_fixture_unavailable' }
    $report.folder_count = @($child.items).Count
    $report.can_attach = $file.can_attach
    $report.can_rename = $file.can_rename
    $detail = Ui 'file' @{ handle = $file.handle }
    if (-not $detail.download_visible) { throw 'native_download_missing' }
    $probeStarted = $true
    Probe 'start' | Out-Null
    $ids = @((Web).command_requests | ForEach-Object request_id)
    $report.stage = 'native_download'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $report.storage = Ui 'download_png_verified' @{
        nameBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($file.name))
        expectedBytes = [string]$file.size_bytes
    }
    $report.download_action_ms = $timer.ElapsedMilliseconds
    $after = Web
    $receipt = @($after.command_requests | Where-Object {
        $_.request_id -notin $ids -and $_.expected_web_action -eq 'download_library_file'
    }) | Select-Object -Last 1
    if ($receipt.status -ne 'succeeded' -or $receipt.result.detail -ne 'download_saved') { throw 'download_receipt_unconfirmed' }
    $report.received_bytes = $after.file_download.received_bytes
    $report.passed = $true
    $report.stage = 'complete'
} catch {
    $errorText = [string]$_.Exception.Message
    $report.error = if ($errorText -match '^[a-z_]+$') { $errorText }
        elseif ($errorText -match '^Semantic UI acceptance failed: ([a-z_]+)$') { $Matches[1] }
        else { 'acceptance_failed' }
} finally {
    if ($probeStarted) {
        try {
            $source = Probe 'file_download_source'
            $report.download_source = $source.receipt.result.detail | ConvertFrom-Json
        } catch { $report.download_source_unavailable = $true }
        try {
            $observed = Probe 'stop'
            $evidence = $observed.receipt.result.detail | ConvertFrom-Json
            $report.protocol = @($evidence.records | Where-Object { $_.path -match '/files/|/estuary/content' } |
                Select-Object method,path,status,requestFields,responseFields)
            $report.protocol_dropped = $evidence.dropped
        } catch { $report.protocol_unavailable = $true }
    }
    try {
        if ($browserOpened) {
            $visible = Ui 'inspect'
            if ($visible.download_status_visible) { Ui 'close_download' | Out-Null }
            elseif ($visible.download_visible) { Ui 'close_detail' | Out-Null }
            if ((Ui 'inspect').library_visible) {
                if ((Web).library_files.directory_handle) { Navigation 'back' | Out-Null }
                Ui 'back' | Out-Null
            }
            Act 'close_chat_side_menu' | Out-Null
        }
        if ($origin) {
            $after = Get-ChatGptWebNativeChatState -Runtime $runtime
            $web = Web
            $report.restored = $after.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                @($after.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                $after.input.text -eq $origin.input.text -and [int]$web.input.official_draft_length -eq 0 -and
                -not $web.streaming -and -not $web.dictation_active
        }
    } catch { $report.restored = $false }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime }
    catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 8 -Compress
}
if (-not $report.passed) { exit 1 }

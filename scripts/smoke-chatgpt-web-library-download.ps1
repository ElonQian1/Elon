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
$report = [ordered]@{ schema = 'elon.chatgpt.library_download_ui.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; cases = @() }
$origin = $null; $browserOpened = $false; $detailOpened = $false; $downloadOpened = $false
$fixtures = @('elon-chatgpt-attachment-fixture-v1.txt', 'elon-chatgpt-media-fixture-v1.png',
    'elon-chatgpt-media-fixture-v1.pdf')

function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
}
function Act([string]$Action) {
    $raw = @(& $runtime.invoke_mcp -Adb $Adb -DeviceSerial $DeviceSerial -Tool ui_control `
        -Arguments (@{ action = $Action } | ConvertTo-Json -Compress) -NoBootstrap `
        -HealthTimeoutSec 3 -RequestTimeoutSec 15 -AdbTimeoutSec 5) | Select-Object -Last 1
    if ($raw.result.isError -or $raw.result.structuredContent.control_ok -ne $true) { throw 'action_unconfirmed' }
}
function Ui([string]$Step, [string]$Handle = '', [string]$Fixture = '') {
    $raw = @(& (Join-Path $PSScriptRoot 'invoke-library-ui-acceptance.ps1') `
        -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial `
        -Step $Step -Handle $Handle -FixtureName $Fixture)
    return $raw[-1] | ConvertFrom-Json
}
function Saved-Fixtures([string]$Name) {
    if ($Name -cnotin $fixtures) { throw 'invalid_fixture_name' }
    $raw = Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments @('shell',
        "find /sdcard/Download -maxdepth 1 -type f -name 'elon-*-$Name'") -TimeoutSec 6
    $pattern = '^/sdcard/Download/elon-[A-Za-z0-9_-]+-' + [regex]::Escape($Name) + '$'
    return @($raw -split "\r?\n" | Where-Object { $_ -cmatch $pattern })
}
function Find-Fixture([string]$Name) {
    Ui 'query_fixture' '' $Name | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $web = Web
        $page = $web.library_files
        if ($page.query -ceq $Name -and $page.stale -ne $true) {
            $matches = @($page.items | Where-Object { $_.name -ceq $Name -and $_.kind -eq 'file' })
            if ($matches.Count -eq 1 -and $matches[0].download_handle) { return $matches[0] }
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'fixed_fixture_unavailable'
}
function Verify-Bytes([string]$Path, [string]$Name, [long]$ExpectedBytes) {
    $root = Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/library-download-bytes'
    New-Item -ItemType Directory -Force -Path $root | Out-Null
    $local = Join-Path $root ([guid]::NewGuid().ToString('N') + '-' + $Name)
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments @('pull', $Path, $local) -TimeoutSec 10 | Out-Null
        $bytes = [IO.File]::ReadAllBytes($local)
        if ($bytes.Length -ne $ExpectedBytes -or $bytes.Length -le 0) { throw 'saved_size_mismatch' }
        $hash = (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant()
        switch ([IO.Path]::GetExtension($Name)) {
            '.txt' { if ($hash -cne '75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3') { throw 'fixture_text_hash_mismatch' } }
            '.png' {
                if ($bytes.Length -lt 33 -or [Convert]::ToHexString($bytes[0..7]) -cne '89504E470D0A1A0A' -or
                    [Convert]::ToHexString($bytes[12..23]) -cne '494844520000020000000180') { throw 'fixture_png_header_mismatch' }
            }
            '.pdf' {
                if ([Text.Encoding]::ASCII.GetString($bytes, 0, 5) -cne '%PDF-' -or
                    [Text.Encoding]::ASCII.GetString($bytes, [Math]::Max(0, $bytes.Length - 1024),
                        [Math]::Min(1024, $bytes.Length)) -notmatch '%%EOF') { throw 'fixture_pdf_header_mismatch' }
            }
        }
        return [ordered]@{ bytes = $bytes.Length; sha256 = $hash; format_valid = $true
            original_hash_verified = $Name.EndsWith('.txt') }
    } finally { if (Test-Path -LiteralPath $local) { Remove-Item -LiteralPath $local -Force } }
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin = Get-ChatGptWebNativeChatState -Runtime $runtime
    $before = Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $before.authenticated -ne $true -or $before.adapter_current -ne $true) { throw 'surface_not_ready' }
    if ($before.streaming -or $before.dictation_active -or $origin.input.text -or
        [int]$before.input.official_draft_length -gt 0 -or $before.file_download.can_cancel) { throw 'existing_work_in_progress' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $report.adapter = $before.adapter_version
    $report.composer_ready_at_start = $before.composer_ready
    $visible = Ui 'inspect'
    $browserOpened = $true
    if (-not $visible.library_visible) { Act 'open_chat_side_menu'; Ui 'browse' | Out-Null }
    foreach ($name in $fixtures) {
        $case = [ordered]@{ format = [IO.Path]::GetExtension($name).Substring(1); passed = $false }
        $report.cases += $case
        $report.stage = 'select_' + $case.format
        $file = Find-Fixture $name
        $savedBefore = @(Saved-Fixtures $name)
        $idsBefore = @((Web).command_requests | ForEach-Object request_id)
        $detailOpened = $true
        $detail = Ui 'file' $file.handle
        if (-not $detail.download_visible) { throw 'rendered_download_missing' }
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $report.stage = 'download_' + $case.format
        $downloadOpened = $true
        Ui 'download' | Out-Null
        $detailOpened = $false
        $uiResult = Ui 'wait_download'
        $web = Web
        $receipt = @($web.command_requests | Where-Object {
            $_.request_id -notin $idsBefore -and $_.expected_web_action -eq 'download_library_file'
        }) | Select-Object -Last 1
        if ($receipt.status -ne 'succeeded' -or $receipt.result.detail -ne 'download_saved' -or
            $uiResult.download_saved -ne $true) { throw 'download_save_unconfirmed' }
        $case.elapsed_ms = $timer.ElapsedMilliseconds
        $case.ui_saved = $true
        $case.download_state = $web.file_download.state
        $created = @(Saved-Fixtures $name | Where-Object { $_ -notin $savedBefore })
        if ($created.Count -ne 1) { throw 'saved_fixture_not_unique' }
        $case.storage = Verify-Bytes $created[0] $name $file.size_bytes
        $case.passed = $true
        Ui 'close_download' | Out-Null
        $downloadOpened = $false
    }
    $report.stage = 'complete'
    $report.passed = $true
} catch {
    $message = [string]$_.Exception.Message
    $report.error = if ($message -match '^[a-z_]+$') { $message } else { 'acceptance_failed' }
} finally {
    try {
        if ($browserOpened) {
            if ($downloadOpened) { Ui 'close_download' | Out-Null }
            if ($detailOpened) { Ui 'close_detail' | Out-Null }
            if ((Ui 'inspect').library_visible) { Ui 'back' | Out-Null }
            Act 'close_chat_side_menu'
        }
        if ($null -ne $origin) {
            $after = Get-ChatGptWebNativeChatState -Runtime $runtime
            $web = Web
            $report.restored = $after.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                $after.input.text -eq $origin.input.text -and
                @($after.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                [int]$web.input.official_draft_length -eq 0 -and -not $web.streaming -and -not $web.dictation_active
        }
    } catch { $report.restored = $false }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime }
    catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 6 -Compress
}
if (-not $report.passed) { exit 1 }

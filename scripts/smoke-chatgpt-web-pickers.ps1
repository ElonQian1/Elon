#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [int]$TimeoutSec = 20
)

$ErrorActionPreference = "Stop"
$invokeMcp = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
$chatGptActivity = 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
$results = [System.Collections.Generic.List[object]]::new()

function Invoke-Adb {
    $serialArgs = if ($DeviceSerial.Trim()) { @("-s", $DeviceSerial.Trim()) } else { @() }
    & $Adb @serialArgs @args
}

function Invoke-ApkMcp {
    param(
        [Parameter(Mandatory = $true)][string]$Tool,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $params = @{
        Adb = $Adb
        DeviceSerial = $DeviceSerial
        Tool = $Tool
        Arguments = ($Arguments | ConvertTo-Json -Depth 20 -Compress)
        OpenAppOnFailure = $true
    }
    if ($EnsureMainActivity) { $params.EnsureMainActivity = $true }
    $response = @(& $invokeMcp @params)
    $structured = $response[-1].result.structuredContent
    if ($null -eq $structured -or $response[-1].result.isError) {
        throw "APK MCP tool failed: $Tool"
    }
    return $structured
}

function Invoke-UiAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $payload = @{} + $Arguments
    $payload.action = $Action
    return Invoke-ApkMcp -Tool "ui_control" -Arguments $payload -EnsureMainActivity:$EnsureMainActivity
}

function Get-TopResumedActivity {
    $line = @(Invoke-Adb shell dumpsys activity activities) |
        Where-Object { $_ -match 'topResumedActivity=' } |
        Select-Object -First 1
    if ($null -eq $line) { return "" }
    return ([string]$line).Trim()
}

function Wait-TopResumedActivity {
    param([Parameter(Mandatory = $true)][scriptblock]$Predicate)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $top = Get-TopResumedActivity
        if (& $Predicate $top) { return $top }
        Start-Sleep -Milliseconds 300
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the expected Android activity. Last top activity: $top"
}

function Wait-ChatGptReady {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Invoke-ApkMcp -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.composer_ready -eq $true
        ) {
            return $state
        }
        Start-Sleep -Milliseconds 300
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT Web readiness."
}

function Wait-ComposerTools {
    param([long]$AfterMs)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Invoke-ApkMcp -Tool "ui_state"
        if (
            $state.last_command.action -eq "collect_composer_tools" -and
            $state.last_command.ok -eq $true -and
            [long]$state.last_command.observed_at_ms -gt $AfterMs
        ) {
            return
        }
        Start-Sleep -Milliseconds 300
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT composer tools."
}

function Get-ToolOption {
    param([Parameter(Mandatory = $true)][string]$Label)

    $before = Invoke-ApkMcp -Tool "ui_state"
    Invoke-UiAction -Action "chatgpt_list_composer_options" -Arguments @{ section = "tools" } | Out-Null
    Wait-ComposerTools -AfterMs ([long]$before.last_command.observed_at_ms)
    $navigation = Invoke-UiAction -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    $option = @($navigation.composer_sections.tools) |
        Where-Object { [string]$_.label -eq $Label } |
        Select-Object -First 1
    if ($null -eq $option) { throw "ChatGPT composer tool is missing: $Label" }
    return $option
}

function Restore-ChatGptActivity {
    for ($attempt = 1; $attempt -le 3; $attempt += 1) {
        if ((Get-TopResumedActivity) -match $chatGptActivity) { return $attempt - 1 }
        Invoke-Adb shell input keyevent 4 | Out-Null
        Start-Sleep -Milliseconds 700
    }
    $top = Get-TopResumedActivity
    if ($top -notmatch $chatGptActivity) { throw "Picker did not return to ChatGPT: $top" }
    return 3
}

if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) { throw "adb not found: $Adb" }
if (-not (Test-Path -LiteralPath $invokeMcp -PathType Leaf)) { throw "Missing APK MCP helper: $invokeMcp" }

$opened = Invoke-UiAction -Action "open_chatgpt_web" -EnsureMainActivity
if ($opened.control_ok -ne $true) { throw "Unable to open ChatGPT Web." }
Wait-TopResumedActivity -Predicate { param($top) $top -match $chatGptActivity } | Out-Null
Wait-ChatGptReady | Out-Null

$cameraLabel = ([string][char]0x76F8) + [char]0x673A
$photoLabel = ([string][char]0x7167) + [char]0x7247
$fileLabel = ([string][char]0x6587) + [char]0x4EF6
$cases = @(
    [pscustomobject]@{ label = $cameraLabel; expected = '(camera|capture)' },
    [pscustomobject]@{ label = $photoLabel; expected = '(picker|document|fileexplorer|photos|gallery)' },
    [pscustomobject]@{ label = $fileLabel; expected = '(picker|document|fileexplorer|files)' }
)

foreach ($case in $cases) {
    $option = Get-ToolOption -Label $case.label
    $selected = Invoke-UiAction -Action "chatgpt_select_composer_option" -Arguments @{
        section = "tools"
        option_id = [string]$option.id
    }
    if ($selected.control_ok -ne $true) { throw "Unable to select ChatGPT tool: $($case.label)" }
    $expectedActivityPattern = [string]$case.expected
    $pickerPredicate = {
        param($top)
        $top -notmatch $chatGptActivity -and $top -match $expectedActivityPattern
    }.GetNewClosure()
    $pickerActivity = Wait-TopResumedActivity -Predicate $pickerPredicate
    $backPresses = Restore-ChatGptActivity
    $state = Invoke-ApkMcp -Tool "ui_state"
    $passed = $state.surface -eq "chatgpt_web" -and
        $state.bridge_state -eq "ready" -and
        @($state.conversation.attachments).Count -eq 0
    $results.Add([pscustomobject]@{
        label = $case.label
        passed = $passed
        picker_activity = $pickerActivity
        return_back_presses = $backPresses
    })
    Write-Output "$(if ($passed) { 'OK' } else { 'FAIL' })`t$($case.label)`t$pickerActivity"
}

$failed = @($results | Where-Object { -not $_.passed })
[ordered]@{
    schema = "elon.chatgpt_web.picker_smoke.v1"
    passed = $failed.Count -eq 0
    device_serial = $DeviceSerial
    selected_local_files = 0
    uploaded_attachments = 0
    results = $results
} | ConvertTo-Json -Depth 10

if ($failed.Count -gt 0) {
    Write-Output "CHATGPT_WEB_PICKER_SMOKE_STATUS=failed"
    exit 1
}
Write-Output "CHATGPT_WEB_PICKER_SMOKE_STATUS=passed"

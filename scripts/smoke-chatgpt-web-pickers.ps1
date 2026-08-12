#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 90)][int]$TimeoutSec = 20,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 64
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$chatGptActivity = 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
$results = [System.Collections.Generic.List[object]]::new()

function Get-TopResumedActivity {
    $output = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "dumpsys", "activity", "activities") -TimeoutSec 10 `
        -Label "read top activity for ChatGPT picker"
    $line = @($output -split "`r?`n") |
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
    throw "Timed out waiting for the expected Android activity."
}

function Wait-CommandReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $ExpectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $ExpectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return [pscustomobject]@{ state = $state; receipt = $receipt }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT command: $ExpectedAction"
}

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{}
    )

    $dispatched = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments -TimeoutSec $TimeoutSec
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Get-ToolOption {
    param([Parameter(Mandatory = $true)][string]$Label)

    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_composer_tools" -Arguments @{ section = "tools" } | Out-Null
    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    $option = @($navigation.composer_sections.tools) |
        Where-Object { [string]$_.label -eq $Label } |
        Select-Object -First 1
    if ($null -eq $option) { throw "ChatGPT composer picker tool is missing." }
    return $option
}

function Restore-ChatGptActivity {
    for ($attempt = 1; $attempt -le 3; $attempt += 1) {
        if ((Get-TopResumedActivity) -match $chatGptActivity) { return $attempt - 1 }
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
            -Label "return from ChatGPT picker" | Out-Null
        Start-Sleep -Milliseconds 700
    }
    if ((Get-TopResumedActivity) -notmatch $chatGptActivity) {
        throw "Picker did not return to ChatGPT."
    }
    return 3
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
        -EnsureMainActivity | Out-Null
    $ready = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(10, $TimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $ready `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    Wait-TopResumedActivity -Predicate { param($top) $top -match $chatGptActivity } | Out-Null

    $cameraLabel = ([string][char]0x76F8) + [char]0x673A
    $photoLabel = ([string][char]0x7167) + [char]0x7247
    $fileLabel = ([string][char]0x6587) + [char]0x4EF6
    $cases = @(
        [pscustomobject]@{ kind = "camera"; label = $cameraLabel; expected = '(camera|capture)' },
        [pscustomobject]@{ kind = "photo"; label = $photoLabel; expected = '(picker|document|fileexplorer|photos|gallery)' },
        [pscustomobject]@{ kind = "file"; label = $fileLabel; expected = '(picker|document|fileexplorer|files)' }
    )

    foreach ($case in $cases) {
        $option = Get-ToolOption -Label $case.label
        $selected = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_select_composer_option" -Arguments @{
                section = "tools"
                option_id = [string]$option.id
            }
        if ($selected.control_ok -ne $true) { throw "Unable to open ChatGPT picker." }
        $expectedActivityPattern = [string]$case.expected
        $pickerPredicate = {
            param($top)
            $top -notmatch $chatGptActivity -and $top -match $expectedActivityPattern
        }.GetNewClosure()
        $pickerActivity = Wait-TopResumedActivity -Predicate $pickerPredicate
        $backPresses = Restore-ChatGptActivity
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $passed = $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            @($state.conversation.attachments).Count -eq 0
        $results.Add([pscustomobject]@{
            kind = [string]$case.kind
            passed = $passed
            picker_activity = $pickerActivity
            return_back_presses = $backPresses
        })
    }

    $failed = @($results | Where-Object { -not $_.passed })
    [ordered]@{
        schema = "elon.chatgpt_web.picker_smoke.v2"
        passed = $failed.Count -eq 0
        device_serial = $DeviceSerial
        adapter_version = [int]$ready.adapter_version
        selected_local_files = 0
        uploaded_attachments = 0
        sent_messages = 0
        cleared_cookies = $false
        cleared_app_data = $false
        results = $results
    } | ConvertTo-Json -Depth 10

    if ($failed.Count -gt 0) {
        throw "ChatGPT Web picker smoke failed: $($failed.Count) case(s)."
    }
    Write-Output "CHATGPT_WEB_PICKER_SMOKE_STATUS=passed"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

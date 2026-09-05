#requires -Version 5.1

function Wait-ChatGptWebSmokeComposerBaseline {
    param(
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [ValidateRange(10, 5000)][int]$PollIntervalMilliseconds = 150
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $stableSamples = 0
    do {
        $state = & $InvokeUiState
        $controls = @($state.ui_manifest.controls)
        $expanded = @($controls | Where-Object { $_.expanded -eq $true })
        $accountOverlay = @($controls | Where-Object {
            $_.region -eq "overlay" -and $_.semantic -in @("settings", "logout")
        })
        if ($state.bridge_state -eq "ready" -and $state.composer_ready -eq $true -and
            $expanded.Count -eq 0 -and $accountOverlay.Count -eq 0) {
            $stableSamples++
            if ($stableSamples -ge 2) { return $state }
        } else {
            $stableSamples = 0
        }
        Start-Sleep -Milliseconds $PollIntervalMilliseconds
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for a stable ChatGPT composer baseline."
}

function Wait-ChatGptWebSmokeComposerOptions {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("model", "tools")][string]$Section,
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][int]$PollIntervalSec,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeNavigation
    )

    $expectedAction = if ($Section -eq "model") { "list_model_options" } else { "list_composer_tools" }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = & $InvokeUiState
        $navigation = & $InvokeNavigation $Section
        $sectionProperty = $navigation.composer_sections.PSObject.Properties[$Section]
        $options = if ($null -eq $sectionProperty) { @() } else { @($sectionProperty.Value) }
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $expectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $expectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return [pscustomobject]@{
                command_state = $state
                receipt = $receipt
                options = $options
            }
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Section composer options request."
}

function Invoke-ChatGptWebSmokeComposerOptions {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("model", "tools")][string]$Section,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][int]$PollIntervalSec,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeAction,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeNavigation
    )

    Wait-ChatGptWebSmokeComposerBaseline -TimeoutSec $TimeoutSec `
        -InvokeUiState $InvokeUiState | Out-Null
    $dispatched = & $InvokeAction "chatgpt_list_composer_options" @{ section = $Section }
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Section composer options." }
    Wait-ChatGptWebSmokeComposerOptions -Section $Section -RequestId $requestId `
        -TimeoutSec $TimeoutSec -PollIntervalSec $PollIntervalSec `
        -InvokeUiState $InvokeUiState -InvokeNavigation $InvokeNavigation
}

function Close-ChatGptWebSmokeComposerOptions {
    param(
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][int]$PollIntervalSec,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeAction,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState
    )

    $dispatched = & $InvokeAction "chatgpt_dismiss_composer_options" @{}
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for composer menu dismissal." }
    Wait-ChatGptCommandReceipt -RequestId $requestId -ExpectedAction "dismiss_composer_menu" `
        -TimeoutSec $TimeoutSec -PollIntervalSec $PollIntervalSec `
        -InvokeUiState $InvokeUiState | Out-Null
}

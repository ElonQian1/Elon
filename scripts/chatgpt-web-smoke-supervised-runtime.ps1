#requires -Version 5.1

function Get-ChatGptWebSmokeConversationPath {
    param([AllowEmptyString()][string]$Url)

    return [regex]::Match($Url, '/c/[A-Za-z0-9_-]{1,160}').Value
}

function Invoke-ChatGptWebSmokeReceiptAction {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{},
        [ValidateRange(10, 600)][int]$TimeoutSec = 90
    )

    $dispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $Runtime `
        -Action $Action -Arguments $Arguments -TimeoutSec $TimeoutSec
    $requestId = [string]$dispatch.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action." }
    return Wait-ChatGptCommandReceipt `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" } `
        -RequestId $requestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $TimeoutSec -PollIntervalSec $Runtime.poll_interval_sec
}

function Start-ChatGptWebSmokeIsolatedConversation {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)]$OriginState,
        [ValidateRange(10, 600)][int]$TimeoutSec = 90
    )

    $originPath = Get-ChatGptWebSmokeConversationPath `
        -Url ([string]$OriginState.conversation.url)
    $originMode = [string]$OriginState.view_mode
    Invoke-ChatGptWebSmokeReceiptAction -Runtime $Runtime `
        -Action "chatgpt_new_conversation" -ExpectedAction "new_conversation" `
        -TimeoutSec $TimeoutSec | Out-Null
    $isolated = Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $TimeoutSec `
        -Description "isolated blank supervised test conversation" -Predicate {
            param($state)
            [int]$state.conversation.message_count -eq 0 -and
                [int]$state.input.text_length -eq 0 -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
        }
    return [pscustomobject]@{
        origin_conversation_path = $originPath
        origin_view_mode = $originMode
        isolated_state = $isolated
    }
}

function Restore-ChatGptWebSmokeOrigin {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [AllowEmptyString()][string]$ConversationPath,
        [Parameter(Mandatory = $true)][string]$ViewMode,
        [ValidateRange(10, 600)][int]$TimeoutSec = 90
    )

    if ($ConversationPath) {
        Invoke-ChatGptWebSmokeReceiptAction -Runtime $Runtime `
            -Action "chatgpt_open_conversation" -ExpectedAction "open_conversation" `
            -Arguments @{ conversation_path = $ConversationPath } `
            -TimeoutSec $TimeoutSec | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $TimeoutSec `
            -Description "original ChatGPT conversation restoration" -Predicate {
                param($state)
                [string]$state.conversation.url -like "*$ConversationPath*" -and
                    $state.bridge_state -eq "ready"
            }.GetNewClosure() | Out-Null
    } else {
        Invoke-ChatGptWebSmokeReceiptAction -Runtime $Runtime `
            -Action "chatgpt_new_conversation" -ExpectedAction "new_conversation" `
            -TimeoutSec $TimeoutSec | Out-Null
    }

    $targetMode = switch ($ViewMode) {
        "native" { "native" }
        "web" { "official" }
        "quick" { "quick" }
        default { throw "Unsupported original ChatGPT view mode." }
    }
    Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $targetMode } | Out-Null
    return Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $TimeoutSec `
        -Description "original ChatGPT view mode restoration" -Predicate {
            param($state)
            [string]$state.view_mode -eq $ViewMode -and $state.bridge_state -eq "ready"
        }.GetNewClosure()
}

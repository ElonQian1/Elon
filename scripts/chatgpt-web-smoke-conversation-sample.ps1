#requires -Version 5.1

function Open-ChatGptWebSmokeConversationSample {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [ValidateRange(10, 600)][int]$TimeoutSec = 90,
        [ValidateRange(0, 100000)][int]$MinimumMessageCount = 0
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        Invoke-ChatGptWebSmokeReadyAction -Runtime $Runtime `
            -Action "chatgpt_list_conversations" -TimeoutSec 15 | Out-Null
        $page = Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
            -Action "chatgpt_get_conversations" -Arguments @{ offset = 0; limit = 10 }
        $paths = @(
            $page.conversations |
                Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.path) } |
                ForEach-Object { [string]$_.path } |
                Select-Object -Unique
        )
        foreach ($path in $paths) {
            $remaining = [int][Math]::Ceiling(
                ($deadline - [DateTimeOffset]::UtcNow).TotalSeconds
            )
            if ($remaining -lt 10) { break }
            $stepTimeout = [Math]::Min($remaining, 20)
            Invoke-ChatGptWebSmokeReceiptAction -Runtime $Runtime `
                -Action "chatgpt_open_conversation" -ExpectedAction "open_conversation" `
                -Arguments @{ conversation_path = $path } `
                -TimeoutSec $stepTimeout | Out-Null
            try {
                $state = Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $stepTimeout `
                    -Description "read-only ChatGPT conversation sample" -Predicate {
                        param($candidate)
                        [string]$candidate.conversation.url -like "*$path*" -and
                            $candidate.bridge_state -eq "ready" -and
                            $candidate.adapter_current -eq $true -and
                            [int]$candidate.conversation.message_count -ge $MinimumMessageCount
                    }.GetNewClosure()
                Start-Sleep -Seconds $Runtime.poll_interval_sec
                $settled = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
                if (
                    $settled.bridge_state -ne "ready" -or
                    $settled.adapter_current -ne $true -or
                    [string]$settled.conversation.url -notlike "*$path*" -or
                    [long]$settled.page_generation -ne [long]$state.page_generation -or
                    [long]$settled.adapter_generation -ne [long]$state.adapter_generation
                ) {
                    continue
                }
                return [pscustomobject]@{
                    path = $path
                    state = $settled
                }
            } catch {
                if ([DateTimeOffset]::UtcNow -ge $deadline) { throw }
            }
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    throw "No existing ChatGPT conversation satisfies the safe sample requirement."
}

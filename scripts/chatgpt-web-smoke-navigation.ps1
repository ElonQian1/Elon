#requires -Version 5.1

function Close-ChatGptWebSmokeNavigation {
    param(
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeAction,
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [ValidateRange(10, 5000)][int]$PollIntervalMilliseconds = 250
    )

    try {
        $dismiss = & $InvokeAction
        if ($dismiss.control_ok -ne $true) {
            return [pscustomobject]@{ passed = $false; detail = [string]$dismiss.action }
        }
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
        do {
            $state = & $InvokeUiState
            $expanded = @(
                $state.ui_manifest.controls |
                    Where-Object { $_.semantic -eq "navigation" -and $_.expanded -eq $true }
            )
            if ($expanded.Count -eq 0) {
                return [pscustomobject]@{ passed = $true; detail = [string]$dismiss.action }
            }
            Start-Sleep -Milliseconds $PollIntervalMilliseconds
        } while ([DateTimeOffset]::UtcNow -lt $deadline)
        return [pscustomobject]@{ passed = $false; detail = "navigation_close_timeout" }
    } catch {
        $detail = if (Get-Command ConvertTo-ChatGptWebSmokeSafeDiagnostic -ErrorAction SilentlyContinue) {
            ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
        } else {
            "navigation_close_failed"
        }
        return [pscustomobject]@{ passed = $false; detail = $detail }
    }
}

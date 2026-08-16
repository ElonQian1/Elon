#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [ValidateRange(15, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

function Get-AppPid {
    $result = Invoke-ElonNativeCommand -FilePath $runtime.adb `
        -ArgumentList @("-s", $runtime.device_serial, "shell", "pidof", "com.elon.app") `
        -TimeoutSeconds 10 -Label "read Elon app pid"
    if ($result.ExitCode -eq 1 -and -not $result.TimedOut) { return "" }
    Assert-ElonNativeCommand -Result $result -FailureMessage "read Elon app pid failed"
    return ([string]$result.Stdout).Trim()
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Assert-NativeReadyState {
    param([Parameter(Mandatory = $true)]$State)

    if ($State.active_surface -ne "social_ai" -or
        [string]$State.social_chat.interaction_mode -ne "chat" -or
        [string]$State.social_chat.web_chat_provider_id -ne "chatgpt_web" -or
        [string]$State.social_chat.web_chat_state -ne "ready" -or
        $State.social_chat.web_chat_composer_ready -ne $true) {
        throw "Native ChatGPT Web AI chat is not ready."
    }
    if ([int]$State.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) {
        throw "Native ChatGPT Web AI adapter version does not match this recovery run."
    }
    if ([int]$State.input.text_length -ne 0) {
        throw "Native composer draft must be empty before session recovery acceptance."
    }
    if ([int]$State.social_chat.web_chat_pending_attachment_count -ne 0 -or
        [string]$State.social_chat.web_chat_attachment_phase -in @("uploading", "sending")) {
        throw "Native attachment send must be idle before session recovery acceptance."
    }
}

function Get-NativeIdentity {
    param([Parameter(Mandatory = $true)]$State)

    Assert-NativeReadyState -State $State
    $conversationPath = [string]$State.social_chat.web_chat_conversation_path
    $messageCount = [int]$State.social_chat.message_count
    if ($messageCount -gt 0 -and [string]::IsNullOrWhiteSpace($conversationPath)) {
        throw "A non-empty native ChatGPT Web AI conversation has no safe restorable path."
    }
    $messageShape = @($State.social_chat.messages) | ForEach-Object {
        $messageId = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.id -MaxLength 220
        $messageRole = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.role -MaxLength 24
        "$messageId|$messageRole|$([int]$_.content_chars)"
    }
    return [pscustomobject]@{
        authenticated = $State.social_chat.web_chat_authenticated -eq $true
        conversation_path = $conversationPath
        message_count = $messageCount
        visible_message_count = @($State.social_chat.messages).Count
        message_shape_sha256 = Get-Sha256 -Value ($messageShape -join "`n")
    }
}

function Get-NativeIdentityKey {
    param([Parameter(Mandatory = $true)]$Identity)

    return @(
        [string]$Identity.authenticated,
        [string]$Identity.conversation_path,
        [string]$Identity.message_count,
        [string]$Identity.visible_message_count,
        [string]$Identity.message_shape_sha256
    ) -join "|"
}

function Wait-StableNativeIdentity {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    $previousKey = ""
    $confirmations = 0
    do {
        $state = Get-ChatGptWebNativeChatState -Runtime $runtime
        try {
            $identity = Get-NativeIdentity -State $state
            $key = Get-NativeIdentityKey -Identity $identity
            if ($key -eq $previousKey) {
                $confirmations++
            } else {
                $previousKey = $key
                $confirmations = 1
            }
            if ($confirmations -ge 3) {
                return [pscustomobject]@{ state = $state; identity = $identity }
            }
        } catch {
            if ($_.Exception.Message -notmatch "not ready") { throw }
            $previousKey = ""
            $confirmations = 0
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Native ChatGPT Web AI conversation did not stabilize before restart."
}

function Wait-NativeIdentity {
    param([Parameter(Mandatory = $true)]$Expected)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Get-ChatGptWebNativeChatState -Runtime $runtime
        try {
            $current = Get-NativeIdentity -State $state
            if ($current.authenticated -eq $Expected.authenticated -and
                $current.conversation_path -eq $Expected.conversation_path -and
                $current.message_count -eq $Expected.message_count -and
                $current.visible_message_count -eq $Expected.visible_message_count -and
                $current.message_shape_sha256 -eq $Expected.message_shape_sha256) {
                return [pscustomobject]@{ state = $state; identity = $current }
            }
        } catch {
            if ($_.Exception.Message -notmatch "not ready") { throw }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Native ChatGPT Web AI conversation did not recover before timeout."
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $ReadyTimeoutSec | Out-Null
    $stableBefore = Wait-StableNativeIdentity
    $beforeState = $stableBefore.state
    $beforeIdentity = $stableBefore.identity
    $beforePid = Get-AppPid
    if ([string]::IsNullOrWhiteSpace($beforePid)) { throw "Elon app pid is unavailable before restart." }

    $restartRequested = $false
    $processStopObserved = $false
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "am", "force-stop", "com.elon.app") -TimeoutSec 15 `
            -Label "force-stop Elon app for native session recovery" | Out-Null
        $restartRequested = $true
        $stoppedDeadline = [DateTimeOffset]::UtcNow.AddSeconds(15)
        do {
            if ([string]::IsNullOrWhiteSpace((Get-AppPid))) { break }
            Start-Sleep -Seconds 1
        } while ([DateTimeOffset]::UtcNow -lt $stoppedDeadline)
        if (-not [string]::IsNullOrWhiteSpace((Get-AppPid))) {
            throw "Elon app process did not stop before native recovery."
        }
        $processStopObserved = $true

        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $ReadyTimeoutSec | Out-Null
        $recovered = Wait-NativeIdentity -Expected $beforeIdentity
        $afterPid = Get-AppPid
        if ([string]::IsNullOrWhiteSpace($afterPid) -or -not $processStopObserved) {
            throw "Elon app process was not recreated."
        }

        $official = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "open_chatgpt_official_fallback" -Arguments @{
                wait_for_target_bind_ms = 12000
            }
        if ($official.target_activity_bound -ne $true) {
            throw "Official fallback did not bind for recovery evidence registration."
        }
        Register-ChatGptWebVerificationCases -Runtime $runtime `
            -CaseIds @("safe/session_recovery") `
            -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 8 `
            -Label "return to native ChatGPT Web AI after recovery evidence" | Out-Null
        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $ReadyTimeoutSec | Out-Null

        [ordered]@{
            schema = "elon.chatgpt_web.native_session_recovery_smoke.v2"
            passed = $true
            native_chat_surface = $true
            process_recreated = $true
            process_stop_observed = $true
            authenticated_state_restored = $recovered.identity.authenticated -eq $beforeIdentity.authenticated
            anonymous_session_supported = -not $beforeIdentity.authenticated
            composer_restored = $recovered.state.social_chat.web_chat_composer_ready -eq $true
            adapter_version_restored = [int]$recovered.state.social_chat.web_chat_adapter_version -eq $ExpectedAdapterVersion
            conversation_identity_restored = $true
            context_window_restored = $true
            private_content_emitted = $false
            sent_messages = 0
            uploaded_attachments = 0
            cleared_cookies = $false
            cleared_app_data = $false
        } | ConvertTo-Json -Depth 5
        Write-Output "CHATGPT_WEB_NATIVE_SESSION_RECOVERY_STATUS=passed"
    } finally {
        if ($restartRequested -and [string]::IsNullOrWhiteSpace((Get-AppPid))) {
            Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $ReadyTimeoutSec | Out-Null
        }
    }
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

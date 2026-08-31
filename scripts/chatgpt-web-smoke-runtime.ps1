#requires -Version 5.1

$nativeCommandModule = Join-Path $PSScriptRoot "native-command-timeout.ps1"
if (-not (Test-Path -LiteralPath $nativeCommandModule -PathType Leaf)) {
    throw "Missing native command timeout helper: $nativeCommandModule"
}
. $nativeCommandModule

function Resolve-ChatGptWebSmokeExpectedAdapterVersion {
    param(
        [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
        [string]$RepositoryRoot = ""
    )

    if ($ExpectedAdapterVersion -gt 0) { return $ExpectedAdapterVersion }
    $root = $RepositoryRoot.Trim()
    if (-not $root) { $root = Split-Path -Parent $PSScriptRoot }
    $adapterPath = Join-Path $root `
        "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt"
    if (-not (Test-Path -LiteralPath $adapterPath -PathType Leaf)) {
        throw "Unable to resolve ChatGPT adapter version; source not found: $adapterPath"
    }
    $source = Get-Content -LiteralPath $adapterPath -Raw
    $match = [regex]::Match($source, 'ADAPTER_VERSION\s*=\s*(\d+)')
    if (-not $match.Success) {
        throw "Unable to resolve ChatGPT adapter version from: $adapterPath"
    }
    return [int]$match.Groups[1].Value
}

function New-ChatGptWebSmokeRuntime {
    param(
        [Parameter(Mandatory = $true)][string]$Adb,
        [Parameter(Mandatory = $true)][string]$DeviceSerial,
        [string]$ExpectedHardwareSerial = "",
        [ValidateRange(1, 10)][int]$PollIntervalSec = 2
    )

    if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) {
        throw "adb not found: $Adb"
    }
    $serial = $DeviceSerial.Trim()
    if (-not $serial) { throw "A device serial is required." }
    if ($serial -match '^emulator-') {
        throw "ChatGPT Web true-device smoke does not accept emulator transport."
    }
    $hardwareSerial = $ExpectedHardwareSerial.Trim()
    if ($serial -match ':\d+$' -and -not $hardwareSerial) {
        throw "Wireless true-device smoke requires an expected hardware serial; use a physical USB serial otherwise."
    }
    $invokeMcp = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
    if (-not (Test-Path -LiteralPath $invokeMcp -PathType Leaf)) {
        throw "Missing APK MCP helper: $invokeMcp"
    }
    return [pscustomobject]@{
        adb = [System.IO.Path]::GetFullPath($Adb)
        device_serial = $serial
        expected_hardware_serial = $hardwareSerial
        invoke_mcp = $invokeMcp
        poll_interval_sec = $PollIntervalSec
        mcp_bootstrapped = $false
        awake_lease_active = $false
        previous_stay_awake_setting = ""
        previous_stay_awake_setting_missing = $false
    }
}

function Assert-ChatGptWebSmokeDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    $devices = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList @("devices", "-l") -TimeoutSeconds 5 -Label "list adb devices"
    Assert-ElonNativeCommand -Result $devices -FailureMessage "Unable to list adb devices"
    $connected = @(
        ([string]$devices.Stdout -split "`r?`n") |
            Where-Object { $_ -match '^\S+\s+device(?:\s|$)' } |
            ForEach-Object { ($_ -split '\s+')[0] }
    )
    if ($Runtime.device_serial -notin $connected) {
        throw "Device is not connected: $($Runtime.device_serial). Verification is deferred."
    }
    $state = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList @("-s", $Runtime.device_serial, "get-state") `
        -TimeoutSeconds 5 -Label "read adb device state"
    Assert-ElonNativeCommand -Result $state -FailureMessage "Unable to read device state"
    if ([string]$state.Stdout -notmatch '^device\s*$') {
        throw "Device is not ready: $($Runtime.device_serial). Verification is deferred."
    }
    $expectedHardwareSerial = [string]$Runtime.expected_hardware_serial
    if ($expectedHardwareSerial) {
        $identity = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
            -ArgumentList @("-s", $Runtime.device_serial, "shell", "getprop", "ro.serialno") `
            -TimeoutSeconds 5 -Label "read adb hardware identity"
        Assert-ElonNativeCommand -Result $identity -FailureMessage "Unable to read device hardware identity"
        if ([string]$identity.Stdout.Trim() -ine $expectedHardwareSerial) {
            throw "Device hardware identity does not match the pinned target. Verification is deferred."
        }
    }
}

function Assert-ChatGptWebSmokeUsbDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    if ($Runtime.device_serial -match ':\d+$') {
        throw "ChatGPT Web USB smoke requires a physical USB serial, not wireless transport."
    }
    Assert-ChatGptWebSmokeDevice -Runtime $Runtime
}

function Assert-ChatGptWebSmokeTrustedDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    Assert-ChatGptWebSmokeDevice -Runtime $Runtime
}

function Assert-ChatGptWebSmokeAdapterVersion {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][ValidateRange(1, 9999)][int]$ExpectedAdapterVersion
    )

    $actual = [int]$State.adapter_version
    if ($actual -ne $ExpectedAdapterVersion) {
        throw "Unexpected ChatGPT adapter version: expected=$ExpectedAdapterVersion actual=$actual."
    }
}

function Get-ChatGptWebSmokeDisplayState {
    param([Parameter(Mandatory = $true)]$Runtime)

    $power = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "power") -TimeoutSec 8 `
        -Label "read device power state"
    $windowPolicy = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "window", "policy") -TimeoutSec 8 `
        -Label "read device keyguard state"
    return [pscustomobject]@{
        awake = $power -match '(?m)^\s*mWakefulness=Awake\s*$'
        keyguard_showing =
            $windowPolicy -match '(?m)^\s*showing=true\s*$' -or
            $windowPolicy -match '(?m)^\s*mIsShowing=true\s*$' -or
            $windowPolicy -match '(?m)\bisStatusBarKeyguard=true\b'
    }
}

function Get-ChatGptWebSmokeUserReadiness {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [switch]$NotifyWhenLocked
    )

    $display = Get-ChatGptWebSmokeDisplayState -Runtime $Runtime
    if (-not $display.keyguard_showing) {
        return [pscustomobject]@{
            schema = "elon.chatgpt_web.user_readiness.v1"
            ready = $true
            status = "ready"
            required_action = ""
            notification_posted = $false
        }
    }

    $notificationPosted = $false
    if ($NotifyWhenLocked) {
        try {
            $attention = Request-ChatGptWebSmokeUserAttention -Runtime $Runtime `
                -Action "unlock_device" `
                -Message "Unlock the phone, then reply in Codex: ready now"
            $notificationPosted = $attention.notification_posted -eq $true
        } catch {
            $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
            Write-Warning "Unable to post ChatGPT Web user-action notification: $detail"
        }
    }

    return [pscustomobject]@{
        schema = "elon.chatgpt_web.user_readiness.v1"
        ready = $false
        status = "user_action_required"
        required_action = "unlock_device"
        notification_posted = $notificationPosted
    }
}

function Request-ChatGptWebSmokeUserAttention {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)]
        [ValidateSet("unlock_device", "attachment", "dictation", "realtime_voice", "sensitive_action")]
        [string]$Action,
        [Parameter(Mandatory = $true)][ValidateLength(1, 120)][string]$Message,
        [switch]$SkipHaptic
    )

    $safeMessage = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $Message -MaxLength 120
    if (-not $safeMessage) { throw "User-attention notification message is empty." }
    Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @(
        "shell", "cmd", "notification", "post",
        "-t", "Codex needs your help",
        "-i", "@android:drawable/stat_sys_warning",
        "codex-chatgpt-$Action",
        $safeMessage
    ) -TimeoutSec 8 -Label "post ChatGPT Web user-action notification" | Out-Null

    $hapticPosted = $false
    if (-not $SkipHaptic) {
        try {
            Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @(
                "shell", "cmd", "vibrator_manager", "synced", "-f", "-B",
                "waveform", "180", "120", "180"
            ) -TimeoutSec 8 -Label "signal ChatGPT Web user action" | Out-Null
            $hapticPosted = $true
        } catch {
            $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
            Write-Warning "Unable to vibrate for ChatGPT Web user action: $detail"
        }
    }

    return [pscustomobject]@{
        schema = "elon.chatgpt_web.user_attention.v1"
        status = "user_action_required"
        required_action = $Action
        notification_posted = $true
        haptic_posted = $hapticPosted
        continuation_requires_explicit_reply = $true
        automatic_sensitive_action = $false
    }
}

function Start-ChatGptWebSmokeAwakeLease {
    param([Parameter(Mandatory = $true)]$Runtime)

    if ($Runtime.awake_lease_active) { return $Runtime }
    $display = Get-ChatGptWebSmokeDisplayState -Runtime $Runtime
    if (-not $display.awake) {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
            -Arguments @("shell", "input", "keyevent", "KEYCODE_WAKEUP") `
            -TimeoutSec 5 -Label "wake ChatGPT Web acceptance device" | Out-Null
        Start-Sleep -Milliseconds 500
        $display = Get-ChatGptWebSmokeDisplayState -Runtime $Runtime
    }
    if (-not $display.awake) {
        throw "Device screen is not awake. Verification is deferred."
    }
    if ($display.keyguard_showing) {
        throw "Device is locked. Unlock it before ChatGPT Web verification; no credential input was attempted."
    }

    $previous = (Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in") `
        -TimeoutSec 5 -Label "read device stay-awake setting").Trim()
    $Runtime.previous_stay_awake_setting = $previous
    $Runtime.previous_stay_awake_setting_missing = -not $previous -or $previous -eq "null"
    Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "settings", "put", "global", "stay_on_while_plugged_in", "7") `
        -TimeoutSec 5 -Label "enable bounded ChatGPT Web stay-awake setting" | Out-Null
    $Runtime.awake_lease_active = $true
    return $Runtime
}

function Stop-ChatGptWebSmokeAwakeLease {
    param([Parameter(Mandatory = $true)]$Runtime)

    if (-not $Runtime.awake_lease_active) { return $true }
    try {
        $arguments = if ($Runtime.previous_stay_awake_setting_missing) {
            @("shell", "settings", "delete", "global", "stay_on_while_plugged_in")
        } else {
            @(
                "shell", "settings", "put", "global", "stay_on_while_plugged_in",
                [string]$Runtime.previous_stay_awake_setting
            )
        }
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments $arguments `
            -TimeoutSec 5 -Label "restore device stay-awake setting" | Out-Null
        $Runtime.awake_lease_active = $false
        return $true
    } catch {
        $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
        Write-Warning "Unable to restore ChatGPT Web stay-awake setting: $detail"
        return $false
    }
}

function Invoke-ChatGptWebSmokeAdb {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [ValidateRange(1, 60)][int]$TimeoutSec = 10,
        [string]$Label = "ChatGPT Web adb command"
    )

    $result = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList (@("-s", $Runtime.device_serial) + $Arguments) `
        -TimeoutSeconds $TimeoutSec -Label $Label
    Assert-ElonNativeCommand -Result $result -FailureMessage "$Label failed"
    return [string]$result.Stdout
}

function Test-ChatGptWebSmokeActivityForeground {
    param([Parameter(Mandatory = $true)]$Runtime)

    $activities = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "activity", "activities") `
        -TimeoutSec 8 -Label "read ChatGPT Web foreground activity"
    return $activities -match
        '(?m)^\s*topResumedActivity=.*com\.elon\.app/\.MainActivity\b'
}

function Test-WebChatNativeChatSurfaceForeground {
    param([Parameter(Mandatory = $true)]$Runtime)

    $activities = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "activity", "activities") `
        -TimeoutSec 8 -Label "read native web chat foreground activity"
    return $activities -match
        '(?m)^\s*topResumedActivity=.*com\.elon\.app/\.MainActivity\b'
}

function ConvertTo-ChatGptWebSmokeSafeDiagnostic {
    param(
        [AllowNull()]$Value,
        [ValidateRange(16, 240)][int]$MaxLength = 120
    )

    $text = ([string]$Value).Replace("`r", " ").Replace("`n", " ").Trim()
    if (-not $text) { return "" }
    $text = [regex]::Replace($text, 'https?://\S+', '<url>', 'IgnoreCase')
    $text = [regex]::Replace(
        $text,
        '(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
        '<email>'
    )
    $text = [regex]::Replace(
        $text,
        '(?i)\b(cookie|token|password|authorization)\s*[:=]\s*\S+',
        '$1=<redacted>'
    )
    if ($text.Length -gt $MaxLength) { $text = $text.Substring(0, $MaxLength) }
    return $text
}

function Get-ChatGptWebSmokeMcpFailureDetail {
    param(
        [AllowNull()]$Response,
        [Parameter(Mandatory = $true)][string]$Tool
    )

    $structured = if ($null -ne $Response) { $Response.result.structuredContent } else { $null }
    $action = if ($null -ne $structured) {
        ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $structured.action -MaxLength 64
    } else { "" }
    $errorValue = if ($null -ne $structured -and $structured.PSObject.Properties["error_code"]) {
        $structured.error_code
    } elseif ($null -ne $structured) {
        $structured.error
    } else { "" }
    $error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $errorValue -MaxLength 120
    $parts = [System.Collections.Generic.List[string]]::new()
    $parts.Add("tool=$Tool")
    if ($action) { $parts.Add("action=$action") }
    if ($error) { $parts.Add("error=$error") }
    return $parts -join " "
}

function Invoke-ChatGptWebSmokeMcp {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Tool,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity,
        [switch]$MainState
    )

    $params = @{
        Adb = $Runtime.adb
        DeviceSerial = $Runtime.device_serial
        Tool = $Tool
        Arguments = ($Arguments | ConvertTo-Json -Depth 20 -Compress)
        HealthTimeoutSec = 15
        RequestTimeoutSec = 30
        AdbTimeoutSec = 8
    }
    if ($EnsureMainActivity) {
        $params.EnsureMainActivity = $true
        $params.OpenAppOnFailure = $true
    }
    if ($Runtime.mcp_bootstrapped) { $params.NoBootstrap = $true }
    try {
        $responses = @(& $Runtime.invoke_mcp @params)
    } catch {
        if (-not $Runtime.mcp_bootstrapped) { throw }
        $Runtime.mcp_bootstrapped = $false
        $params.Remove("NoBootstrap")
        $params.Remove("OpenAppOnFailure")
        $responses = @(& $Runtime.invoke_mcp @params)
    }
    $response = $responses | Select-Object -Last 1
    if ($null -eq $response -or $response.result.isError) {
        $detail = Get-ChatGptWebSmokeMcpFailureDetail -Response $response -Tool $Tool
        throw "APK MCP tool failed: $detail"
    }
    $structured = $response.result.structuredContent
    if ($null -eq $structured) {
        throw "APK MCP tool returned no structured content: $Tool"
    }
    $Runtime.mcp_bootstrapped = $true
    if (
        $Tool -eq "ui_state" -and
        -not $MainState -and
        $null -ne $structured.chatgpt_web_mcp
    ) {
        return $structured.chatgpt_web_mcp
    }
    return $structured
}

function Invoke-ChatGptWebSmokeAction {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $payload = @{} + $Arguments
    $payload.action = $Action
    return Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_control" `
        -Arguments $payload -EnsureMainActivity:$EnsureMainActivity
}

function Invoke-ChatGptWebSmokeReadyAction {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{},
        [ValidateRange(1, 300)][int]$TimeoutSec = 30,
        [switch]$EnsureMainActivity
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true
        ) {
            try {
                return Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action $Action `
                    -Arguments $Arguments -EnsureMainActivity:$EnsureMainActivity
            } catch {
                # The MCP action gate rejects both states before creating a command receipt,
                # so retrying them cannot duplicate an already-dispatched web command.
                if (
                    $_.Exception.Message -notmatch
                        "bridge_not_ready|adapter_generation_not_ready"
                ) { throw }
            }
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting to dispatch ChatGPT Web action: $Action"
}

function Register-ChatGptWebVerificationCases {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string[]]$CaseIds,
        [Parameter(Mandatory = $true)][ValidateRange(1, 9999)][int]$ExpectedAdapterVersion,
        [switch]$ProductionSurface
    )

    $expected = @($CaseIds | Sort-Object -Unique)
    $arguments = @{
        case_ids = $expected
        expected_adapter_version = $ExpectedAdapterVersion
    }
    $result = if ($ProductionSurface) {
        Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
            -Action "chatgpt_record_verification_cases" -Arguments $arguments
    } else {
        Invoke-ChatGptWebSmokeReadyAction -Runtime $Runtime `
            -Action "chatgpt_record_verification_cases" -Arguments $arguments
    }
    $recorded = @($result.recorded_case_ids | Sort-Object -Unique)
    if (($recorded -join "`n") -ne ($expected -join "`n")) {
        throw "ChatGPT Web verification evidence did not record the requested cases."
    }
    $current = @($result.verification_evidence.current_case_ids)
    foreach ($caseId in $expected) {
        if ($caseId -notin $current) {
            throw "ChatGPT Web verification evidence is stale after registration: $caseId"
        }
    }
    return $result
}

function Open-ChatGptWebSmokeSurface {
    param([Parameter(Mandatory = $true)]$Runtime)

    Open-WebChatNativeChatSurface -Runtime $Runtime -ProviderId "chatgpt_web" | Out-Null
    return Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
}

function Open-WebChatNativeChatSurface {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)]
        [ValidateSet("chatgpt_web", "google_web")]
        [string]$ProviderId,
        [ValidateRange(10, 180)][int]$TimeoutSec = 90
    )

    $providerName = if ($ProviderId -eq "google_web") { "Google Web AI" } else { "ChatGPT Web AI" }
    $opened = Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
        -Action "open_social_ai_chat" -Arguments @{
            wait_for_target_bind_ms = 12000
        } -EnsureMainActivity
    if ($opened.control_ok -ne $true) {
        throw "Unable to open the social AI chat surface."
    }
    $selected = Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
        -Action "select_web_chat_provider" -Arguments @{ provider_id = $ProviderId }
    if ($selected.control_ok -ne $true) {
        throw "Unable to select $providerName in the native chat surface."
    }

    return Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $TimeoutSec -MainState `
        -Description "ready $providerName native chat surface" -Predicate {
            param($state)
            $state.active_surface -eq "social_ai" -and
                [string]$state.social_chat.interaction_mode -eq "chat" -and
                [string]$state.social_chat.web_chat_provider_id -eq $ProviderId -and
                [string]$state.social_chat.web_chat_state -eq "ready" -and
                $state.social_chat.web_chat_composer_ready -eq $true
        }.GetNewClosure()
}

function Open-ChatGptWebNativeChatSurface {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [ValidateRange(10, 180)][int]$TimeoutSec = 90
    )

    return Open-WebChatNativeChatSurface -Runtime $Runtime `
        -ProviderId "chatgpt_web" -TimeoutSec $TimeoutSec
}

function Restore-WebChatNativeConversation {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)]
        [ValidateSet("chatgpt_web", "google_web")]
        [string]$ProviderId,
        [Parameter(Mandatory = $true)][string]$ConversationPath,
        [ValidateRange(5, 120)][int]$TimeoutSec = 45
    )

    if ([string]::IsNullOrWhiteSpace($ConversationPath)) { return $false }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $dispatched = $false
    do {
        try {
            $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState
            if (
                [string]$state.social_chat.web_chat_provider_id -eq $ProviderId -and
                [string]$state.social_chat.web_chat_conversation_path -eq $ConversationPath
            ) {
                return $true
            }
            if (-not $dispatched) {
                Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
                    -Action "open_web_chat_conversation" `
                    -Arguments @{ conversation_path = $ConversationPath } | Out-Null
                $dispatched = $true
            }
        } catch {
            $Runtime.mcp_bootstrapped = $false
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $false
}

function Get-ChatGptWebNativeChatState {
    param([Parameter(Mandatory = $true)]$Runtime)

    $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState
    if ($state.active_surface -ne "social_ai") {
        throw "The native social AI chat surface is not active."
    }
    return $state
}

function Restore-ChatGptWebSmokeInteractiveBaseline {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [ValidateRange(10, 180)][int]$TimeoutSec = 60,
        [ValidateRange(1, 5)][int]$MaxBackAttempts = 3
    )

    Open-ChatGptWebSmokeSurface -Runtime $Runtime | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $backAttempts = 0
    $blankConversationRequested = $false
    $last = $null
    do {
        $last = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        if ($last.login_required -eq $true -or [string]$last.page_kind -eq "auth") {
            throw "ChatGPT Web interactive baseline requires an authenticated session."
        }
        $blockingControls = @(
            @($last.ui_manifest.controls) |
                Where-Object { [string]$_.region -in @("overlay", "dialog") }
        )
        $interactivePage = [string]$last.page_kind -in @("conversation", "home")
        if (
            $last.surface -eq "chatgpt_web" -and
            $last.bridge_state -eq "ready" -and
            $last.adapter_current -eq $true -and
            $last.authenticated -eq $true -and
            $last.composer_ready -eq $true -and
            $interactivePage -and
            $blockingControls.Count -eq 0
        ) {
            return [pscustomobject]@{
                schema = "elon.chatgpt_web.interactive_baseline.v1"
                ready = $true
                recovery = if ($blankConversationRequested) {
                    "blank_conversation"
                } elseif ($backAttempts -gt 0) {
                    "back_navigation"
                } else {
                    "none"
                }
                back_attempts = $backAttempts
                blank_conversation_requested = $blankConversationRequested
            }
        }

        if (
            $blockingControls.Count -gt 0 -or
            [string]$last.page_kind -eq "feature"
        ) {
            if ($backAttempts -lt $MaxBackAttempts) {
                Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
                    -Arguments @("shell", "input", "keyevent", "4") `
                    -TimeoutSec 8 -Label "restore ChatGPT Web interactive baseline" | Out-Null
                $backAttempts += 1
                Start-Sleep -Seconds $Runtime.poll_interval_sec
                if (-not (Test-ChatGptWebSmokeActivityForeground -Runtime $Runtime)) {
                    Open-ChatGptWebSmokeSurface -Runtime $Runtime | Out-Null
                }
                continue
            }
            if (-not $blankConversationRequested) {
                Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
                    -Action "chatgpt_new_conversation" | Out-Null
                $blankConversationRequested = $true
                Start-Sleep -Seconds $Runtime.poll_interval_sec
                continue
            }
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    $lastPage = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $last.page_kind -MaxLength 32
    throw "Timed out restoring ChatGPT Web interactive baseline. Last page=$lastPage."
}

function Wait-ChatGptWebSmokeAuthenticatedReady {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [ValidateRange(10, 300)][int]$TimeoutSec = 90,
        [ValidateRange(1, 60)][int]$InitialWaitSec = 15
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $ready = {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true
    }
    try {
        return Wait-ChatGptWebSmokeState -Runtime $Runtime `
            -TimeoutSec ([Math]::Min($InitialWaitSec, $TimeoutSec)) `
            -Description "authenticated ChatGPT Web bridge" -Predicate $ready
    } catch {
        # Native state is cache-first; do not turn a slow identity-layer resume into
        # a destructive page reload. The bounded recovery coordinator remains authoritative.
        $remaining = [int][Math]::Ceiling(
            ($deadline - [DateTimeOffset]::UtcNow).TotalSeconds
        )
        if ($remaining -lt 1) {
            throw "Timed out resuming the authenticated ChatGPT Web bridge."
        }
        return Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $remaining `
            -Description "resumed authenticated ChatGPT Web bridge" -Predicate $ready
    }
}

function Wait-ChatGptWebSmokeState {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][scriptblock]$Predicate,
        [Parameter(Mandatory = $true)][ValidateRange(1, 300)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][string]$Description,
        [switch]$RequireChatGptForeground,
        [switch]$MainState
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        if ($RequireChatGptForeground -and -not (Test-ChatGptWebSmokeActivityForeground -Runtime $Runtime)) {
            throw "ChatGPT Web acceptance was interrupted because another app took the foreground."
        }
        $last = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState:$MainState
        if (& $Predicate $last) { return $last }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description. Last page=$($last.page_kind), bridge=$($last.bridge_state)."
}

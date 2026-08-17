#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)]
    [ValidateSet("PrepareAndRemove", "StageForSend", "SendAndVerifyReply")]
    [string]$Phase,
    [switch]$UserConfirmedAttachmentSend,
    [string]$CheckpointPath = "",
    [ValidateRange(10, 240)][int]$TimeoutSec = 120,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
$fixtureId = "fixed_ascii_text_v1"
$checkpointSchema = "elon.chatgpt_web.native_attachment_checkpoint.v1"
$reportSchema = "elon.chatgpt_web.native_attachment_smoke.v1"
if (-not $CheckpointPath.Trim()) {
    $CheckpointPath = Join-Path (Split-Path $PSScriptRoot -Parent) `
        ".ai-tmp\chatgpt-web-native-attachment.json"
}
$CheckpointPath = [System.IO.Path]::GetFullPath($CheckpointPath)

function Write-Checkpoint {
    param([Parameter(Mandatory = $true)]$Value)

    $directory = Split-Path $CheckpointPath -Parent
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $temporary = "$CheckpointPath.$PID.$([Guid]::NewGuid().ToString('N')).tmp"
    try {
        [System.IO.File]::WriteAllText(
            $temporary,
            "$(($Value | ConvertTo-Json -Depth 8))`n",
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Read-Checkpoint {
    if (-not (Test-Path -LiteralPath $CheckpointPath -PathType Leaf)) {
        throw "Native attachment checkpoint is missing. Run -Phase PrepareAndRemove first."
    }
    if ((Get-Item -LiteralPath $CheckpointPath).Length -gt 16384) {
        throw "Native attachment checkpoint exceeds the safe size limit."
    }
    $value = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
    if ([string]$value.schema -ne $checkpointSchema) {
        throw "Native attachment checkpoint schema is not supported."
    }
    $created = [DateTimeOffset]::Parse([string]$value.created_utc)
    if ([DateTimeOffset]::UtcNow - $created -gt [TimeSpan]::FromHours(12)) {
        throw "Native attachment checkpoint expired. Run -Phase PrepareAndRemove again."
    }
    return $value
}

function Get-DeviceBinding {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($ExpectedHardwareSerial.Trim())
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Assert-Checkpoint {
    param([Parameter(Mandatory = $true)]$Checkpoint)

    if ([string]$Checkpoint.device_binding_sha256 -ne (Get-DeviceBinding)) {
        throw "Native attachment checkpoint belongs to another physical device."
    }
    if ([int]$Checkpoint.adapter_version -ne $ExpectedAdapterVersion) {
        throw "Native attachment checkpoint adapter version is stale."
    }
}

function Get-NativeState {
    return Get-ChatGptWebNativeChatState -Runtime $runtime
}

function Wait-NativeState {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Predicate,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $nativePredicate = $Predicate
    $surfacePredicate = {
        param($state)
        $state.active_surface -eq "social_ai" -and (& $nativePredicate $state)
    }.GetNewClosure()
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec -MainState `
        -Description $Description -Predicate $surfacePredicate
}

function Invoke-NativeAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{}
    )

    $result = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
    if ($result.control_ok -ne $true) {
        $errorCode = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $result.error -MaxLength 80
        throw "Native ChatGPT Web AI action failed: action=$Action error=$errorCode"
    }
    return $result
}

function Assert-ReadyForAttachment {
    param([Parameter(Mandatory = $true)]$State)

    if ([string]$State.social_chat.web_chat_state -ne "ready" -or
        $State.social_chat.web_chat_composer_ready -ne $true) {
        throw "Native ChatGPT Web AI composer is not ready."
    }
    if ($State.social_chat.web_chat_authenticated -ne $true) {
        throw "Native attachment acceptance requires an authenticated ChatGPT Web session."
    }
    if ([int]$State.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) {
        throw "Native ChatGPT Web AI adapter version does not match this acceptance run."
    }
    if ($State.social_chat.web_chat_attachment_supported -ne $true) {
        throw "The current ChatGPT Web session does not support attachments."
    }
    if ([int]$State.input.text_length -ne 0) {
        throw "Native composer draft must be empty before attachment acceptance."
    }
}

function Assert-FixtureState {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][bool]$Expected
    )

    $fixture = $State.chatgpt_web_acceptance_attachment
    if ($null -eq $fixture -or $fixture.fixture_staged -ne $Expected) {
        throw "Pinned native attachment fixture state did not match the expectation."
    }
    $expectedCount = if ($Expected) { 1 } else { 0 }
    if ([int]$fixture.composer_pending_count -ne $expectedCount) {
        throw "Unexpected native pending attachment count."
    }
    if ($fixture.local_only -ne $true -or $fixture.upload_started -ne $false) {
        throw "Fixture preparation unexpectedly started an upload."
    }
}

function Restore-Origin {
    param([Parameter(Mandatory = $true)]$Checkpoint)

    $origin = [string]$Checkpoint.origin_conversation_path
    if ($origin) {
        Invoke-NativeAction -Action "open_web_chat_conversation" `
            -Arguments @{ conversation_path = $origin } | Out-Null
        return Wait-NativeState -Description "restored native ChatGPT Web AI conversation" -Predicate {
            param($state)
            [string]$state.social_chat.web_chat_conversation_path -eq $origin
        }.GetNewClosure()
    }
    return Get-NativeState
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $ready = Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec
    Assert-ReadyForAttachment -State $ready

    switch ($Phase) {
        "PrepareAndRemove" {
            Assert-FixtureState -State $ready -Expected $false
            $originPath = [string]$ready.social_chat.web_chat_conversation_path
            if ([int]$ready.social_chat.message_count -gt 0 -and -not $originPath) {
                throw "Current non-empty ChatGPT Web AI conversation cannot be restored safely."
            }
            if (-not $originPath -and [int]$ready.social_chat.message_count -eq 0) {
                $isolated = $ready
            } else {
                Invoke-NativeAction -Action "start_new_web_chat_conversation" | Out-Null
                $isolated = Wait-NativeState `
                    -Description "isolated blank native ChatGPT Web AI conversation" `
                    -Predicate {
                        param($state)
                        [int]$state.social_chat.message_count -eq 0 -and
                            [string]$state.social_chat.web_chat_state -eq "ready" -and
                            [string]$state.social_chat.web_chat_conversation_path -ne $originPath
                    }.GetNewClosure()
            }
            Assert-ReadyForAttachment -State $isolated
            Invoke-NativeAction -Action "stage_chatgpt_web_acceptance_attachment" `
                -Arguments @{ fixture_id = $fixtureId } | Out-Null
            $staged = Get-NativeState
            Assert-FixtureState -State $staged -Expected $true
            Invoke-NativeAction -Action "remove_chatgpt_web_acceptance_attachment" `
                -Arguments @{ fixture_id = $fixtureId } | Out-Null
            $removed = Get-NativeState
            Assert-FixtureState -State $removed -Expected $false
            $checkpoint = [ordered]@{
                schema = $checkpointSchema
                phase = "fixture_removed"
                created_utc = [DateTimeOffset]::UtcNow.ToString("o")
                updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                device_binding_sha256 = Get-DeviceBinding
                adapter_version = $ExpectedAdapterVersion
                origin_conversation_path = $originPath
                isolated_conversation_path = [string]$isolated.social_chat.web_chat_conversation_path
                marker = "ELON-NATIVE-ATTACHMENT-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
                sent_messages = 0
                cleared_cookies = $false
                cleared_app_data = $false
            }
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "fixture_removed"
                passed = $true
                native_chat_surface = $true
                fixed_fixture_staged = 1
                fixed_fixture_removed = 1
                uploaded_attachments = 0
                sent_messages = 0
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_NATIVE_ATTACHMENT_STATUS=fixture_removed"
        }
        "StageForSend" {
            $checkpoint = Read-Checkpoint
            Assert-Checkpoint -Checkpoint $checkpoint
            if ([string]$checkpoint.phase -ne "fixture_removed") {
                throw "Native attachment checkpoint is not ready for send staging."
            }
            if ([int]$ready.social_chat.message_count -ne 0 -or
                [string]$ready.social_chat.web_chat_conversation_path -ne
                    [string]$checkpoint.isolated_conversation_path) {
                throw "The isolated native ChatGPT Web AI conversation changed. Prepare again."
            }
            Assert-FixtureState -State $ready -Expected $false
            Invoke-NativeAction -Action "stage_chatgpt_web_acceptance_attachment" `
                -Arguments @{ fixture_id = $fixtureId } | Out-Null
            $staged = Get-NativeState
            Assert-FixtureState -State $staged -Expected $true
            $checkpoint.phase = "fixture_staged_for_send"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint -Value $checkpoint
            Write-Output "CHATGPT_WEB_NATIVE_ATTACHMENT_STATUS=staged_for_supervised_send"
        }
        "SendAndVerifyReply" {
            if (-not $UserConfirmedAttachmentSend) {
                throw "Use -UserConfirmedAttachmentSend only while the user supervises the upload and message send."
            }
            $checkpoint = Read-Checkpoint
            Assert-Checkpoint -Checkpoint $checkpoint
            if ([string]$checkpoint.phase -eq "send_dispatching") {
                throw "Attachment send outcome is ambiguous after interruption; inspect the native chat before retrying."
            }
            if ([string]$checkpoint.phase -notin @("fixture_staged_for_send", "reply_requested")) {
                throw "Native attachment fixture has not been staged for send."
            }
            $marker = [string]$checkpoint.marker
            if ([string]$checkpoint.phase -eq "fixture_staged_for_send") {
                if ([int]$ready.social_chat.message_count -ne 0 -or
                    [string]$ready.social_chat.web_chat_conversation_path -ne
                        [string]$checkpoint.isolated_conversation_path) {
                    throw "The isolated native ChatGPT Web AI conversation changed before send."
                }
                Assert-FixtureState -State $ready -Expected $true
                Invoke-NativeAction -Action "set_input_text" -Arguments @{
                    text = "Read the attached test file and reply only with: $marker"
                } | Out-Null
                $checkpoint.phase = "send_dispatching"
                $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                Write-Checkpoint -Value $checkpoint
                Invoke-NativeAction -Action "send_input" | Out-Null
                $checkpoint.phase = "reply_requested"
                $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                Write-Checkpoint -Value $checkpoint
            }
            try {
                $completed = Wait-NativeState -Description "native attachment reply" -Predicate {
                    param($state)
                    $messages = @($state.social_chat.messages)
                    $reply = $messages | Where-Object {
                        [string]$_.role -eq "friend" -and [string]$_.content -like "*$marker*"
                    } | Select-Object -Last 1
                    [string]$state.social_chat.web_chat_attachment_phase -eq "completed" -and
                        [int]$state.social_chat.web_chat_pending_attachment_count -eq 0 -and
                        $null -ne $reply
                }.GetNewClosure()
            } catch {
                $failureState = Get-NativeState
                if ([string]$failureState.social_chat.web_chat_attachment_phase -eq "failed") {
                    Invoke-NativeAction -Action "remove_chatgpt_web_acceptance_attachment" `
                        -Arguments @{ fixture_id = $fixtureId } | Out-Null
                    $checkpoint.phase = "fixture_removed"
                    $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                    Write-Checkpoint -Value $checkpoint
                    throw "Native attachment upload failed; fixed fixture removed. Run StageForSend to retry."
                }
                throw
            }
            Invoke-NativeAction -Action "remove_chatgpt_web_acceptance_attachment" `
                -Arguments @{ fixture_id = $fixtureId } | Out-Null
            $restored = Restore-Origin -Checkpoint $checkpoint
            $official = Invoke-NativeAction -Action "open_chatgpt_official_fallback"
            if ($official.target_activity_bound -ne $true) {
                throw "Official fallback did not bind for evidence registration."
            }
            Register-ChatGptWebVerificationCases -Runtime $runtime `
                -CaseIds @("supervised/attachment_lifecycle") `
                -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") `
                -TimeoutSec 8 -Label "return to native ChatGPT Web AI chat" | Out-Null
            Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
            $checkpoint.phase = "passed"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            $checkpoint.sent_messages = 1
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "passed"
                passed = $true
                native_chat_surface = $true
                fixed_fixture_uploaded = 1
                assistant_completed = $true
                original_conversation_restored = [string]::IsNullOrWhiteSpace(
                    [string]$checkpoint.origin_conversation_path
                ) -or [string]$restored.social_chat.web_chat_conversation_path -eq
                    [string]$checkpoint.origin_conversation_path
                sent_messages = 1
                cleared_cookies = $false
                cleared_app_data = $false
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_NATIVE_ATTACHMENT_STATUS=passed"
        }
    }
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

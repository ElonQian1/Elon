#requires -Version 7.0

param(
    [string]$AdbPath = "",
    [string]$DeviceSerial = "",
    [string]$PackageName = "com.duoguan.football",
    [string]$OutputDir = "",
    [string]$EvidencePath = "",
    [string]$Tester = "codex-main-project-adb",
    [string]$MainProjectCommit = "",
    [int]$LogcatLines = 2500,
    [switch]$CaptureHoldGesture,
    [int]$HoldStartX = 540,
    [int]$HoldStartY = 2305,
    [int]$HoldEndX = 170,
    [int]$HoldEndY = 2050,
    [int]$HoldDurationMs = 7000,
    [switch]$MarkFinalReady,
    [switch]$ObservedVoiceComposerView,
    [switch]$ObservedTextVoiceToggle,
    [switch]$ObservedHoldToTalkButton,
    [switch]$ObservedRecordingOverlay,
    [switch]$ObservedSlideToCancel,
    [switch]$ObservedZoneSend,
    [switch]$ObservedZoneAiReply,
    [switch]$ObservedZoneTranscribe,
    [switch]$ObservedTooShort,
    [switch]$ObservedSystemAsrSuccess,
    [switch]$ObservedSystemAsrTimeoutServerFallback,
    [switch]$ObservedServerAsrSuccess,
    [switch]$ObservedServerAsrFailureRecoversUi,
    [switch]$ObservedTtsPlayback,
    [switch]$ObservedAsrTtsFreeWithZeroAiBalance
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    $root = git rev-parse --show-toplevel 2>$null
    if (-not $root) {
        throw "Run this script inside the main project git repository."
    }
    return $root.Trim()
}

function Resolve-AdbPath {
    param([string]$RequestedPath)

    if ($RequestedPath) {
        if (-not (Test-Path -LiteralPath $RequestedPath)) {
            throw "ADB path not found: $RequestedPath"
        }
        return (Resolve-Path -LiteralPath $RequestedPath).Path
    }

    $defaultPath = "D:\Android\sdk\platform-tools\adb.exe"
    if (Test-Path -LiteralPath $defaultPath) {
        return $defaultPath
    }

    $cmd = Get-Command adb -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    throw "ADB not found. Pass -AdbPath or install Android platform-tools."
}

function Resolve-AdbDevice {
    param(
        [string]$Adb,
        [string]$RequestedSerial
    )

    if ($RequestedSerial) {
        return $RequestedSerial
    }

    $lines = & $Adb devices | Select-Object -Skip 1
    $devices = @(
        $lines |
            Where-Object { $_ -match "\sdevice(\s|$)" } |
            ForEach-Object { ($_ -split "\s+")[0] } |
            Where-Object { $_ }
    )
    if ($devices.Count -eq 0) {
        throw "No online adb device found."
    }
    if ($devices.Count -gt 1) {
        throw "Multiple adb devices found. Pass -DeviceSerial. Devices: $($devices -join ', ')"
    }
    return $devices[0]
}

function Invoke-AdbText {
    param(
        [string]$Adb,
        [string]$Serial,
        [string[]]$AdbArgs
    )

    $allArgs = @()
    if ($Serial) {
        $allArgs += @("-s", $Serial)
    }
    $allArgs += $AdbArgs
    $output = & $Adb @allArgs 2>&1
    return ($output -join "`n").Trim()
}

function Save-AdbScreenshot {
    param(
        [string]$Adb,
        [string]$Serial,
        [string]$Path
    )

    $adbArgs = @()
    if ($Serial) {
        $adbArgs += @("-s", $Serial)
    }
    $adbArgs += @("exec-out", "screencap", "-p")
    & $Adb @adbArgs > $Path
}

function Save-AdbUiDump {
    param(
        [string]$Adb,
        [string]$Serial,
        [string]$RemotePath,
        [string]$LocalPath
    )

    for ($attempt = 1; $attempt -le 3; $attempt += 1) {
        Invoke-AdbText -Adb $Adb -Serial $Serial -AdbArgs @("shell", "rm", "-f", $RemotePath) | Out-Null
        Invoke-AdbText -Adb $Adb -Serial $Serial -AdbArgs @("shell", "uiautomator", "dump", $RemotePath) | Out-Null
        Start-Sleep -Milliseconds (250 * $attempt)
        $remoteCheck = Invoke-AdbText -Adb $Adb -Serial $Serial -AdbArgs @("shell", "ls", "-l", $RemotePath)
        if ($remoteCheck -match [regex]::Escape($RemotePath) -and $remoteCheck -notmatch "No such file|No such file or directory") {
            $adbArgs = @()
            if ($Serial) {
                $adbArgs += @("-s", $Serial)
            }
            $adbArgs += @("pull", $RemotePath, $LocalPath)
            & $Adb @adbArgs | Out-Null
            if (Test-Path -LiteralPath $LocalPath) {
                return $true
            }
        }
    }

    Write-Warning "Unable to capture UI dump: $RemotePath"
    return $false
}

function Get-PackageField {
    param(
        [string]$Dump,
        [string]$Pattern
    )

    $match = [regex]::Match($Dump, $Pattern)
    if ($match.Success) {
        return $match.Groups[1].Value.Trim()
    }
    return ""
}

function Test-Text {
    param(
        [string]$Text,
        [string]$Pattern
    )

    return ($Text -match $Pattern)
}

function Read-OptionalTextFile {
    param([string]$Path)

    if ($Path -and (Test-Path -LiteralPath $Path)) {
        return Get-Content -Raw -Path $Path
    }
    return ""
}

function ConvertTo-CheckValue {
    param(
        [bool]$AutoValue,
        [switch]$Observed
    )

    return [bool]($AutoValue -or $Observed)
}

function New-Artifact {
    param(
        [string]$Type,
        [string]$Ref,
        [string]$Note
    )

    return [ordered]@{
        type = $Type
        ref = $Ref
        note = $Note
    }
}

$repoRoot = Resolve-RepoRoot
$adb = Resolve-AdbPath -RequestedPath $AdbPath
$serial = Resolve-AdbDevice -Adb $adb -RequestedSerial $DeviceSerial

if (-not $OutputDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot "target\fb2-voice-device-evidence\$stamp"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$outputDirFull = (Resolve-Path -LiteralPath $OutputDir).Path

if (-not $EvidencePath) {
    $EvidencePath = Join-Path $outputDirFull "voice-device-evidence.json"
}

if (-not $MainProjectCommit) {
    $MainProjectCommit = (git rev-parse --short HEAD).Trim()
}

$captureId = [guid]::NewGuid().ToString("N")
$screenBefore = Join-Path $outputDirFull "screen-before.png"
$uiBefore = Join-Path $outputDirFull "ui-before.xml"
$logcatPath = Join-Path $outputDirFull "voice-logcat.txt"

Save-AdbScreenshot -Adb $adb -Serial $serial -Path $screenBefore
$uiBeforeCaptured = Save-AdbUiDump -Adb $adb -Serial $serial -RemotePath "/sdcard/fb2-voice-$captureId-before.xml" -LocalPath $uiBefore

$duringScreen = ""
$duringUi = ""
$afterScreen = ""
$afterUi = ""

if ($CaptureHoldGesture) {
    Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("logcat", "-c") | Out-Null

    $job = Start-Job -ScriptBlock {
        param($AdbPath, $Serial, $StartX, $StartY, $EndX, $EndY, $DurationMs)
        $args = @()
        if ($Serial) {
            $args += @("-s", $Serial)
        }
        $args += @("shell", "input", "swipe", $StartX, $StartY, $EndX, $EndY, $DurationMs)
        & $AdbPath @args
    } -ArgumentList $adb, $serial, $HoldStartX, $HoldStartY, $HoldEndX, $HoldEndY, $HoldDurationMs

    Start-Sleep -Seconds 2
    $duringScreen = Join-Path $outputDirFull "screen-during-hold.png"
    $duringUi = Join-Path $outputDirFull "ui-during-hold.xml"
    Save-AdbScreenshot -Adb $adb -Serial $serial -Path $duringScreen
    $duringUiCaptured = Save-AdbUiDump -Adb $adb -Serial $serial -RemotePath "/sdcard/fb2-voice-$captureId-during.xml" -LocalPath $duringUi

    Wait-Job $job | Out-Null
    Receive-Job $job | Out-Null
    Remove-Job $job

    Start-Sleep -Seconds 5
    $afterScreen = Join-Path $outputDirFull "screen-after-release.png"
    $afterUi = Join-Path $outputDirFull "ui-after-release.xml"
    Save-AdbScreenshot -Adb $adb -Serial $serial -Path $afterScreen
    $afterUiCaptured = Save-AdbUiDump -Adb $adb -Serial $serial -RemotePath "/sdcard/fb2-voice-$captureId-after.xml" -LocalPath $afterUi
}

Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("logcat", "-d", "-t", [string]$LogcatLines) | Set-Content -Path $logcatPath -Encoding UTF8

$manufacturer = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "getprop", "ro.product.manufacturer")
$model = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "getprop", "ro.product.model")
$release = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "getprop", "ro.build.version.release")
$sdk = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "getprop", "ro.build.version.sdk")
$speechService = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "settings", "get", "secure", "voice_recognition_service")
$packageDump = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "dumpsys", "package", $PackageName)
$appOps = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "cmd", "appops", "get", $PackageName, "RECORD_AUDIO")
$windowFocus = Invoke-AdbText -Adb $adb -Serial $serial -AdbArgs @("shell", "dumpsys", "window")

$beforeText = Read-OptionalTextFile -Path $uiBefore
$duringText = Read-OptionalTextFile -Path $duringUi
$afterText = Read-OptionalTextFile -Path $afterUi
$allUiText = "$beforeText`n$duringText`n$afterText"
$logcatText = Get-Content -Raw -Path $logcatPath

$hasHoldToTalk = Test-Text $allUiText "按住\s*说话"
$hasRecordingOverlay = Test-Text $duringText "正在听|准备中|录音中"
$hasZoneSend = Test-Text $duringText "发送"
$hasZoneAiReply = Test-Text $duringText "AI回复"
$hasZoneTranscribe = Test-Text $duringText "转文字"
$hasCancel = Test-Text $duringText "取消"
$afterRecovered = (-not $CaptureHoldGesture) -or ((Test-Text $afterText "按住\s*说话") -and (-not (Test-Text $afterText "识别中|准备中")))

$systemAsrSuccessLogHint = Test-Text $logcatText "onResults|finalResult|SpeechRecognizer.*RESULT|ASR_RESULT"
$serverFallbackLogHint = Test-Text $logcatText "onVoiceServerFallbackStarted|SERVER_PROCESSING|/api/voice/asr|voice/asr"
$serverAsrSuccessLogHint = Test-Text $logcatText "serverAsrSuccess|SERVER_ASR_SUCCESS|Whisper.*success|/api/voice/asr.*(200|OK)"
$serverAsrFailureRecoveryLogHint = (Test-Text $logcatText "serverAsrFailure|SERVER_ASR_ERROR|Whisper.*fail|语音识别服务暂不可用") -and $afterRecovered
$ttsPlaybackLogHint = Test-Text $logcatText "ttsStart|ttsEnd|TextToSpeech|TTS"

$checks = [ordered]@{
    usesVoiceComposerView = ConvertTo-CheckValue -AutoValue ($hasHoldToTalk -and ($hasRecordingOverlay -or $hasZoneAiReply -or $hasZoneTranscribe)) -Observed:$ObservedVoiceComposerView
    textVoiceToggle = ConvertTo-CheckValue -AutoValue (Test-Text $allUiText "发送消息|按住\s*说话") -Observed:$ObservedTextVoiceToggle
    holdToTalkButton = ConvertTo-CheckValue -AutoValue $hasHoldToTalk -Observed:$ObservedHoldToTalkButton
    recordingOverlay = ConvertTo-CheckValue -AutoValue $hasRecordingOverlay -Observed:$ObservedRecordingOverlay
    slideToCancel = ConvertTo-CheckValue -AutoValue ($CaptureHoldGesture -and $hasCancel -and $afterRecovered) -Observed:$ObservedSlideToCancel
    zoneSend = ConvertTo-CheckValue -AutoValue $hasZoneSend -Observed:$ObservedZoneSend
    zoneAiReply = ConvertTo-CheckValue -AutoValue $hasZoneAiReply -Observed:$ObservedZoneAiReply
    zoneTranscribe = ConvertTo-CheckValue -AutoValue $hasZoneTranscribe -Observed:$ObservedZoneTranscribe
    tooShort = [bool]$ObservedTooShort
    # ASR/TTS 核心项不能只靠模糊 log 自动放行，必须由测试者结合 artifact 显式确认。
    systemAsrSuccess = [bool]$ObservedSystemAsrSuccess
    systemAsrTimeoutServerFallback = [bool]$ObservedSystemAsrTimeoutServerFallback
    serverAsrSuccess = [bool]$ObservedServerAsrSuccess
    serverAsrFailureRecoversUi = [bool]$ObservedServerAsrFailureRecoversUi
    ttsPlayback = [bool]$ObservedTtsPlayback
    asrTtsFreeWithZeroAiBalance = [bool]$ObservedAsrTtsFreeWithZeroAiBalance
}

$missingChecks = @(
    $checks.GetEnumerator() |
        Where-Object { -not [bool]$_.Value } |
        ForEach-Object { $_.Key }
)
if ($MarkFinalReady -and $missingChecks.Count -gt 0) {
    throw "Cannot set finalAcceptanceReady=true. Missing checks: $($missingChecks -join ', ')"
}

$artifacts = @(
    New-Artifact -Type "screenshot" -Ref "screen-before.png" -Note "Current fb2 screen before evidence capture."
)
if ($uiBeforeCaptured) {
    $artifacts += New-Artifact -Type "ui_dump" -Ref "ui-before.xml" -Note "UIAutomator dump before evidence capture."
}
if ($duringScreen) {
    $artifacts += New-Artifact -Type "screenshot" -Ref "screen-during-hold.png" -Note "Hold-to-talk gesture capture while finger is down."
    if ($duringUiCaptured) {
        $artifacts += New-Artifact -Type "ui_dump" -Ref "ui-during-hold.xml" -Note "UIAutomator dump while hold-to-talk overlay is visible."
    }
}
if ($afterScreen) {
    $artifacts += New-Artifact -Type "screenshot" -Ref "screen-after-release.png" -Note "fb2 screen after release/cancel, used to prove UI recovery."
    if ($afterUiCaptured) {
        $artifacts += New-Artifact -Type "ui_dump" -Ref "ui-after-release.xml" -Note "UIAutomator dump after release/cancel."
    }
}
$artifacts += New-Artifact -Type "logcat" -Ref "voice-logcat.txt" -Note "ADB logcat captured around the voice evidence run."

$permissionLine = Get-PackageField -Dump $packageDump -Pattern "android\.permission\.RECORD_AUDIO:\s*([^\r\n]+)"
$versionName = Get-PackageField -Dump $packageDump -Pattern "versionName=([^\s\r\n]+)"
$versionCode = Get-PackageField -Dump $packageDump -Pattern "versionCode=([0-9]+)"
$lastUpdateTime = Get-PackageField -Dump $packageDump -Pattern "lastUpdateTime=([^\r\n]+)"

$observations = @(
    "ADB device $serial captured by collect-fb2-voice-device-evidence.ps1.",
    "Package $PackageName versionName=$versionName versionCode=$versionCode.",
    "RECORD_AUDIO package permission: $permissionLine.",
    "RECORD_AUDIO appops: $appOps.",
    "System speech recognizer service: $speechService.",
    "Window focus contains fb2: $($windowFocus -match [regex]::Escape($PackageName))."
)
if ($CaptureHoldGesture) {
    $observations += "CaptureHoldGesture used swipe from ($HoldStartX,$HoldStartY) to ($HoldEndX,$HoldEndY) for ${HoldDurationMs}ms."
    $observations += "After gesture recovered to hold-to-talk without processing stuck: $afterRecovered."
}
$observations += "Log hint systemAsrSuccess=$systemAsrSuccessLogHint serverFallback=$serverFallbackLogHint serverAsrSuccess=$serverAsrSuccessLogHint serverAsrFailureRecovery=$serverAsrFailureRecoveryLogHint ttsPlayback=$ttsPlaybackLogHint."
if ($missingChecks.Count -gt 0) {
    $observations += "Not final-ready. Missing checks: $($missingChecks -join ', ')."
}

$evidence = [ordered]@{
    schema = "fb2.voice_device_evidence.v1"
    finalAcceptanceReady = [bool]$MarkFinalReady
    recordedAt = Get-Date -Format "yyyy-MM-ddTHH:mm:sszzz"
    tester = $Tester
    device = [ordered]@{
        manufacturer = $manufacturer
        model = $model
        osVersion = "Android $release"
        androidApi = if ($sdk -match "^\d+$") { [int]$sdk } else { $sdk }
        speechRecognizerService = $speechService
    }
    apk = [ordered]@{
        packageName = $PackageName
        versionName = $versionName
        versionCode = if ($versionCode -match "^\d+$") { [int]$versionCode } else { $versionCode }
        lastUpdateTime = $lastUpdateTime
        recordAudioPermission = $permissionLine
        recordAudioAppOps = $appOps
    }
    sdk = [ordered]@{
        mainProjectCommit = $MainProjectCommit
        voiceKit = "android/chat-voice-kit"
        bootstrapApi = "VoiceComposerBootstrap.applyFb2GroupChatConfig(...)"
    }
    checks = $checks
    observations = $observations
    artifacts = $artifacts
    remainingForFinalAcceptance = $missingChecks
}

$evidence | ConvertTo-Json -Depth 8 | Set-Content -Path $EvidencePath -Encoding UTF8

Write-Output "OK`tvoice evidence collected`t$EvidencePath"
Write-Output "OK`tvoice evidence output dir`t$outputDirFull"
Write-Output "OK`tvoice evidence final ready`tfinalAcceptanceReady=$([bool]$MarkFinalReady)"
if ($missingChecks.Count -gt 0) {
    Write-Output "WARN`tvoice evidence missing checks`t$($missingChecks -join ',')"
}

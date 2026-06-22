#requires -Version 7.0

function Resolve-Fb2VoiceEvidenceArtifactPath {
    param(
        [string]$Ref,
        [string]$EvidenceFilePath,
        [string]$RepoRoot
    )

    if ([string]::IsNullOrWhiteSpace($Ref)) {
        return ""
    }
    if ($Ref -match '^https?://') {
        return $Ref
    }
    if ([System.IO.Path]::IsPathRooted($Ref)) {
        return $Ref
    }

    $evidenceDir = Split-Path -Parent (Resolve-Path -LiteralPath $EvidenceFilePath).Path
    $candidate = Join-Path $evidenceDir $Ref
    if (Test-Path -LiteralPath $candidate) {
        return $candidate
    }

    Join-Path $RepoRoot $Ref
}

function Test-Fb2VoiceEvidencePlaceholderArtifactRef {
    param([string]$Ref)

    if ([string]::IsNullOrWhiteSpace($Ref)) {
        return $true
    }
    $lower = $Ref.ToLowerInvariant()
    return $lower.Contains("example") `
        -or $lower.Contains("placeholder") `
        -or $lower.Contains("saved file path") `
        -or $lower.Contains("screenshot/video path") `
        -or $lower.Contains("adb logcat excerpt")
}

function Assert-Fb2VoiceDeviceEvidence {
    param(
        [object]$Evidence,
        [string]$EvidenceFilePath,
        [string]$RepoRoot,
        [string]$MinFb2ApkVersion
    )

    Assert-True ($Evidence.schema -eq "fb2.voice_device_evidence.v1") "voice evidence schema" "$($Evidence.schema)"
    Assert-JsonBool $Evidence "finalAcceptanceReady" "voice evidence final ready"
    Assert-NonEmptyField $Evidence "recordedAt" "voice evidence recordedAt"
    Assert-NonEmptyField $Evidence "tester" "voice evidence tester"
    Assert-NonEmptyField $Evidence.device "manufacturer" "voice evidence device manufacturer"
    Assert-NonEmptyField $Evidence.device "model" "voice evidence device model"
    Assert-NonEmptyField $Evidence.device "osVersion" "voice evidence OS"
    Assert-NonEmptyField $Evidence.device "speechRecognizerService" "voice evidence recognizer"
    Assert-NonEmptyField $Evidence.apk "versionName" "voice evidence APK versionName"
    Assert-True ((Compare-VersionParts ([string]$Evidence.apk.versionName) $MinFb2ApkVersion) -ge 0) "voice evidence APK version" "version=$($Evidence.apk.versionName) min=$MinFb2ApkVersion"
    Assert-NonEmptyField $Evidence.apk "versionCode" "voice evidence APK versionCode"
    Assert-NonEmptyField $Evidence.sdk "mainProjectCommit" "voice evidence main project commit"
    Assert-JsonBool $Evidence.checks "usesVoiceComposerView" "voice evidence uses VoiceComposerView"
    Assert-JsonBool $Evidence.checks "textVoiceToggle" "voice evidence text/voice toggle"
    Assert-JsonBool $Evidence.checks "holdToTalkButton" "voice evidence hold-to-talk"
    Assert-JsonBool $Evidence.checks "recordingOverlay" "voice evidence recording overlay"
    Assert-JsonBool $Evidence.checks "slideToCancel" "voice evidence slide cancel"
    Assert-JsonBool $Evidence.checks "zoneSend" "voice evidence send zone"
    Assert-JsonBool $Evidence.checks "zoneAiReply" "voice evidence AI reply zone"
    Assert-JsonBool $Evidence.checks "zoneTranscribe" "voice evidence transcribe zone"
    Assert-JsonBool $Evidence.checks "tooShort" "voice evidence too short"
    Assert-JsonBool $Evidence.checks "systemAsrSuccess" "voice evidence system ASR success"
    Assert-JsonBool $Evidence.checks "systemAsrTimeoutServerFallback" "voice evidence ASR timeout fallback"
    Assert-JsonBool $Evidence.checks "serverAsrSuccess" "voice evidence server ASR success"
    Assert-JsonBool $Evidence.checks "serverAsrFailureRecoversUi" "voice evidence server ASR failure recovery"
    Assert-JsonBool $Evidence.checks "ttsPlayback" "voice evidence TTS playback"
    Assert-JsonBool $Evidence.checks "asrTtsFreeWithZeroAiBalance" "voice evidence ASR/TTS free"

    $artifactItems = @($Evidence.artifacts | Where-Object { $_ })
    $artifactCount = $artifactItems.Count
    Assert-True ($artifactCount -gt 0) "voice evidence artifacts" "count=$artifactCount"

    $artifactTypes = @()
    $validArtifactRefs = 0
    foreach ($artifact in $artifactItems) {
        $type = ([string]$artifact.type).Trim()
        $ref = ([string]$artifact.ref).Trim()
        if ($type) {
            $artifactTypes += $type.ToLowerInvariant()
        }
        Assert-True (-not [string]::IsNullOrWhiteSpace($type)) "voice evidence artifact type" $type
        Assert-True (-not (Test-Fb2VoiceEvidencePlaceholderArtifactRef $ref)) "voice evidence artifact ref" $ref
        $resolvedRef = Resolve-Fb2VoiceEvidenceArtifactPath -Ref $ref -EvidenceFilePath $EvidenceFilePath -RepoRoot $RepoRoot
        if ($resolvedRef -match '^https?://') {
            $validArtifactRefs += 1
            Pass "voice evidence artifact url" $resolvedRef
        } else {
            $exists = [bool](Test-Path -LiteralPath $resolvedRef)
            Assert-True $exists "voice evidence artifact exists" $resolvedRef
            if ($exists) {
                $validArtifactRefs += 1
            }
        }
    }
    Assert-True ($validArtifactRefs -eq $artifactCount) "voice evidence artifact refs complete" "valid=$validArtifactRefs count=$artifactCount"
    $hasLogcat = @($artifactTypes | Where-Object { $_ -eq "logcat" -or $_ -like "*log*" }).Count -gt 0
    $hasVisual = @($artifactTypes | Where-Object { $_ -eq "screenshot" -or $_ -eq "video" -or $_ -eq "screenshot_or_video" -or $_ -like "*screen*" -or $_ -like "*video*" }).Count -gt 0
    Assert-True $hasLogcat "voice evidence artifact logcat" ($artifactTypes -join ",")
    Assert-True $hasVisual "voice evidence artifact visual" ($artifactTypes -join ",")
}

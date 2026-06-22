#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$MainToken = "",
    [string]$Fb2Base = "",
    [string]$Fb2Token = "",
    [string]$Fb2UserToken = "",
    [string]$Fb2Username = "",
    [string]$Fb2Password = "",
    [string]$GroupId = "official",
    [string]$ExternalUserId = "",
    [int]$RequestTimeoutSec = 45,
    [int]$RetryCount = 1,
    [switch]$SelfTest,
    [switch]$FinalAcceptance,
    [switch]$DataOnlyAcceptance,
    [switch]$IncludePlatformOrderSummary,
    [switch]$RequireFb2Live,
    [switch]$RequireAllScenarios,
    [switch]$CheckFb2ApkVersion,
    [string]$MinFb2ApkVersion = "1.1.48",
    [string]$ExpectedFb2UpdateKind = "full_apk",
    [switch]$CheckLocalVoiceSdkBuild,
    [string]$VoiceSdkGradleTask = ":chat-voice-kit:assembleDebug",
    [switch]$RequireVoiceDeviceEvidence,
    [string]$VoiceDeviceEvidencePath = "",
    [switch]$CheckQuality,
    [switch]$RequireFeedbackCoverage,
    [switch]$CheckPermissionBoundaries,
    [string]$QualitySince = "",
    [string]$QualityUntil = "",
    [double]$MaxLargeContextPackRate = 0.75,
    [double]$MaxCitationUnmatchedRate = 0,
    [double]$MaxMissingContextRate = 0,
    [double]$MaxWrongContextRate = 0,
    [int]$MinFeedbackCount = 0,
    [int]$MinMatchedCitedSourceCount = 0,
    [int]$MinNonSyntheticFeedbackCount = 1,
    [int]$MinOpinionAdoptionCount = 1,
    [int]$QualityFeedbackSampleLimit = 5,
    [switch]$RequireNonSyntheticQualityReadiness,
    [switch]$CheckDomainProjection,
    [switch]$SkipVoiceContractChecks,
    [switch]$RequireNoSkips
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) {
    $MainBase = $env:ELON_MAIN_BASE
}
if (-not $MainBase) {
    $MainBase = "http://43.139.149.158:8080"
}
if (-not $MainToken) {
    $MainToken = $env:ELON_MAIN_TOKEN
}
if (-not $Fb2Base) {
    $Fb2Base = $env:FB2_API_BASE
}
if (-not $Fb2Base) {
    $Fb2Base = "http://123.207.48.146:8080"
}
if (-not $Fb2Token) {
    $Fb2Token = $env:FB2_AI_CENTER_TOKEN
}
if (-not $Fb2UserToken) {
    $Fb2UserToken = $env:FB2_USER_TOKEN
}
if (-not $Fb2Username) {
    $Fb2Username = $env:FB2_VISIBLE_SMOKE_USERNAME
}
if (-not $Fb2Password) {
    $Fb2Password = $env:FB2_VISIBLE_SMOKE_PASSWORD
}

$MainBase = $MainBase.TrimEnd("/")
$Fb2Base = $Fb2Base.TrimEnd("/")
$amp = [char]38
$script:Failed = 0
$script:Skipped = 0

if ($FinalAcceptance) {
    $IncludePlatformOrderSummary = $true
    $RequireFb2Live = $true
    $RequireAllScenarios = $true
    $CheckFb2ApkVersion = $true
    $CheckLocalVoiceSdkBuild = $true
    $RequireVoiceDeviceEvidence = $true
    $CheckQuality = $true
    $RequireFeedbackCoverage = $true
    $RequireNonSyntheticQualityReadiness = $true
    $CheckPermissionBoundaries = $true
    $RequireNoSkips = $true
}

if ($DataOnlyAcceptance) {
    $IncludePlatformOrderSummary = $true
    $RequireFb2Live = $true
    $RequireAllScenarios = $true
    $CheckFb2ApkVersion = $true
    $CheckQuality = $true
    $RequireFeedbackCoverage = $true
    $RequireNonSyntheticQualityReadiness = $true
    $CheckPermissionBoundaries = $true
    $RequireNoSkips = $true
    $SkipVoiceContractChecks = $true
}

if ($RequireAllScenarios) {
    $CheckDomainProjection = $true
}

if ($RequireFeedbackCoverage) {
    if (-not $PSBoundParameters.ContainsKey("MinFeedbackCount")) {
        $MinFeedbackCount = 1
    }
    if (-not $PSBoundParameters.ContainsKey("MinMatchedCitedSourceCount")) {
        $MinMatchedCitedSourceCount = 1
    }
}

$qualityCheckRequested = $CheckQuality `
    -or $RequireFeedbackCoverage `
    -or [bool]$QualitySince `
    -or [bool]$QualityUntil `
    -or $PSBoundParameters.ContainsKey("MaxLargeContextPackRate") `
    -or $PSBoundParameters.ContainsKey("MaxCitationUnmatchedRate") `
    -or $PSBoundParameters.ContainsKey("MaxMissingContextRate") `
    -or $PSBoundParameters.ContainsKey("MaxWrongContextRate") `
    -or $PSBoundParameters.ContainsKey("MinFeedbackCount") `
    -or $PSBoundParameters.ContainsKey("MinMatchedCitedSourceCount") `
    -or $PSBoundParameters.ContainsKey("MinNonSyntheticFeedbackCount") `
    -or $PSBoundParameters.ContainsKey("MinOpinionAdoptionCount") `
    -or $RequireNonSyntheticQualityReadiness

$permissionCheckRequested = $CheckPermissionBoundaries

function Write-Check {
    param(
        [string]$Status,
        [string]$Name,
        [string]$Detail = ""
    )
    if ($Detail) {
        Write-Output "$Status`t$Name`t$Detail"
    } else {
        Write-Output "$Status`t$Name"
    }
}

function Pass {
    param([string]$Name, [string]$Detail = "")
    Write-Check "OK" $Name $Detail
}

function Skip {
    param([string]$Name, [string]$Detail = "")
    $script:Skipped += 1
    Write-Check "SKIP" $Name $Detail
}

function Fail {
    param([string]$Name, [string]$Detail = "")
    $script:Failed += 1
    Write-Check "FAIL" $Name $Detail
}

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Name,
        [string]$Detail = ""
    )
    if ($Condition) {
        Pass $Name $Detail
    } else {
        Fail $Name $Detail
    }
}

function Encode-QueryValue {
    param([string]$Value)
    [System.Uri]::EscapeDataString($Value)
}

function Add-QueryParam {
    param(
        [System.Collections.Generic.List[string]]$Params,
        [string]$Name,
        [string]$Value
    )
    if ($Value) {
        $Params.Add("$Name=$(Encode-QueryValue $Value)") | Out-Null
    }
}

function Get-ItemCount {
    param([object]$Value)
    if ($null -eq $Value) {
        return 0
    }
    return @($Value).Count
}

function Assert-MinCount {
    param(
        [object]$Value,
        [int]$Minimum,
        [string]$Name
    )
    $count = Get-ItemCount $Value
    Assert-True ($count -ge $Minimum) $Name "count=$count min=$Minimum"
}

function Assert-ContainsValue {
    param(
        [object]$Value,
        [string]$Expected,
        [string]$Name
    )
    Assert-True (@($Value) -contains $Expected) $Name $Expected
}

function Get-ObjectPropertyNames {
    param([object]$Value)
    if ($null -eq $Value) {
        return @()
    }
    @($Value.PSObject.Properties | ForEach-Object { $_.Name })
}

function Get-Fb2ManifestToolIds {
    param([object]$Manifest)

    $ids = @()
    $ids += @($Manifest.data.tool_ids)
    $ids += @($Manifest.data.tool_contract.tool_ids)
    foreach ($endpoint in @($Manifest.data.tool_contract.endpoints)) {
        if ($endpoint -is [string]) {
            $ids += $endpoint
            continue
        }
        foreach ($field in @("id", "tool_id", "name", "key")) {
            $property = $endpoint.PSObject.Properties[$field]
            if ($null -ne $property -and -not [string]::IsNullOrWhiteSpace([string]$property.Value)) {
                $ids += [string]$property.Value
                break
            }
        }
    }
    @($ids | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
}

function Compare-VersionParts {
    param(
        [string]$Actual,
        [string]$Minimum
    )
    $actualParts = @($Actual -split '[^0-9]+' | Where-Object { $_ -ne "" } | ForEach-Object { [int]$_ })
    $minimumParts = @($Minimum -split '[^0-9]+' | Where-Object { $_ -ne "" } | ForEach-Object { [int]$_ })
    $max = [Math]::Max($actualParts.Count, $minimumParts.Count)
    for ($i = 0; $i -lt $max; $i += 1) {
        $a = if ($i -lt $actualParts.Count) { $actualParts[$i] } else { 0 }
        $m = if ($i -lt $minimumParts.Count) { $minimumParts[$i] } else { 0 }
        if ($a -gt $m) { return 1 }
        if ($a -lt $m) { return -1 }
    }
    return 0
}

function Resolve-AbsoluteUrl {
    param([string]$Url)
    if ($Url -match '^https?://') {
        return $Url
    }
    if ($Url.StartsWith("/")) {
        return "$Fb2Base$Url"
    }
    return "$Fb2Base/$Url"
}

function Assert-JsonBool {
    param(
        [object]$Value,
        [string]$Field,
        [string]$Name
    )
    $property = if ($null -ne $Value) { $Value.PSObject.Properties[$Field] } else { $null }
    $actual = if ($null -ne $property) { $property.Value } else { $null }
    $isStrictTrue = ($null -ne $property) -and ($actual -is [bool]) -and ($actual -eq $true)
    Assert-True $isStrictTrue $Name "$Field=$actual"
}

function Assert-NonEmptyField {
    param(
        [object]$Value,
        [string]$Field,
        [string]$Name
    )
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$Value.$Field)) $Name "$Field=$($Value.$Field)"
}

function Find-EvalScenario {
    param(
        [object[]]$Scenarios,
        [string]$Id
    )
    @($Scenarios | Where-Object { $_.id -eq $Id } | Select-Object -First 1)[0]
}

function Find-ProjectionPermission {
    param(
        [object[]]$Permissions,
        [string]$Data
    )
    @($Permissions | Where-Object { $_.data -eq $Data } | Select-Object -First 1)[0]
}

function Assert-ScenarioContains {
    param(
        [object]$Scenario,
        [string]$Field,
        [string[]]$ExpectedValues,
        [string]$Name
    )
    $actual = @($Scenario.$Field)
    $missing = @($ExpectedValues | Where-Object { $actual -notcontains $_ })
    Assert-True (@($missing).Count -eq 0) $Name "missing=$($missing -join ',')"
}

function Invoke-Json {
    param(
        [string]$Url,
        [hashtable]$Headers = @{},
        [string]$Method = "GET",
        [object]$Body = $null
    )
    $attempt = 0
    while ($true) {
        $attempt += 1
        $params = @{
            Uri = $Url
            Method = $Method
            Headers = $Headers
            TimeoutSec = $RequestTimeoutSec
        }
        if ($null -ne $Body) {
            $params["ContentType"] = "application/json"
            $params["Body"] = ($Body | ConvertTo-Json -Depth 8 -Compress)
        }
        try {
            return Invoke-RestMethod @params
        } catch {
            if ($attempt -ge ($RetryCount + 1)) {
                throw
            }
            Start-Sleep -Seconds ([Math]::Min(2 * $attempt, 5))
        }
    }
}

function Invoke-HttpStatus {
    param(
        [string]$Url,
        [hashtable]$Headers = @{},
        [string]$Method = "GET",
        [object]$Body = $null
    )
    $params = @{
        Uri = $Url
        Method = $Method
        Headers = $Headers
        TimeoutSec = $RequestTimeoutSec
        SkipHttpErrorCheck = $true
    }
    if ($null -ne $Body) {
        $params["ContentType"] = "application/json"
        $params["Body"] = ($Body | ConvertTo-Json -Depth 8 -Compress)
    }
    Invoke-WebRequest @params
}

function Assert-StatusCode {
    param(
        [object]$Response,
        [int]$Expected,
        [string]$Name
    )
    $actual = [int]$Response.StatusCode
    Assert-True ($actual -eq $Expected) $Name "status=$actual expected=$Expected"
}

function Get-NestedNumber {
    param(
        [object]$Value,
        [string[]]$Paths
    )
    foreach ($path in $Paths) {
        $current = $Value
        $found = $true
        foreach ($part in ($path -split "\.")) {
            if ($null -eq $current) {
                $found = $false
                break
            }
            $property = $current.PSObject.Properties[$part]
            if ($null -eq $property) {
                $found = $false
                break
            }
            $current = $property.Value
        }
        if ($found -and $null -ne $current -and "$current" -ne "") {
            return [double]$current
        }
    }
    return $null
}

function Fb2-Headers {
    param(
        [string]$UserId = "",
        [bool]$PlatformScope = $false
    )
    $headers = @{ "X-FB2-AI-CENTER-TOKEN" = $Fb2Token.Trim() }
    if ($UserId) {
        $headers["X-FB2-AI-CONTEXT-USER-ID"] = $UserId
    }
    if ($PlatformScope) {
        $headers["X-FB2-AI-CONTEXT-SCOPE"] = "platform_order_summary"
    }
    $headers
}

function Get-MismatchedFb2UserId {
    param([string]$UserId)
    $candidate = "00000000-0000-0000-0000-000000000001"
    if ($UserId -eq $candidate) {
        return "00000000-0000-0000-0000-000000000002"
    }
    $candidate
}

$voiceEvidenceHelper = Join-Path $PSScriptRoot "fb2-ai-center-voice-evidence.ps1"
. $voiceEvidenceHelper
$contextProjectionHelper = Join-Path $PSScriptRoot "fb2-ai-center-context-projection.ps1"
. $contextProjectionHelper

function New-VoiceEvidenceSelfTestObject {
    param([object[]]$Artifacts)

    [pscustomobject]@{
        schema = "fb2.voice_device_evidence.v1"
        finalAcceptanceReady = $true
        recordedAt = "2026-06-22T10:00:00+08:00"
        tester = "smoke-self-test"
        device = [pscustomobject]@{
            manufacturer = "Xiaomi"
            model = "23116PN5BC"
            osVersion = "HyperOS OS3.0"
            androidApi = 35
            speechRecognizerService = "com.xiaomi.mibrain.speech/.asr.AsrService"
        }
        apk = [pscustomobject]@{
            packageName = "com.duoguan.football"
            versionName = "1.1.48"
            versionCode = 96
        }
        sdk = [pscustomobject]@{
            mainProjectCommit = "selftest"
            voiceKit = "android/chat-voice-kit"
            bootstrapApi = "VoiceComposerBootstrap.applyFb2GroupChatConfig(...)"
        }
        checks = [pscustomobject]@{
            usesVoiceComposerView = $true
            textVoiceToggle = $true
            holdToTalkButton = $true
            recordingOverlay = $true
            slideToCancel = $true
            zoneSend = $true
            zoneAiReply = $true
            zoneTranscribe = $true
            tooShort = $true
            systemAsrSuccess = $true
            systemAsrTimeoutServerFallback = $true
            serverAsrSuccess = $true
            serverAsrFailureRecoversUi = $true
            ttsPlayback = $true
            asrTtsFreeWithZeroAiBalance = $true
        }
        artifacts = @($Artifacts)
    }
}

function Copy-SelfTestObject {
    param([object]$Value)
    $Value | ConvertTo-Json -Depth 12 | ConvertFrom-Json
}

function Invoke-VoiceEvidenceSelfTestCase {
    param(
        [string]$Name,
        [object]$Evidence,
        [string]$EvidenceFilePath,
        [string]$RepoRoot,
        [bool]$ShouldPass
    )

    Set-Content -Path $EvidenceFilePath -Value ($Evidence | ConvertTo-Json -Depth 12) -Encoding UTF8
    $before = $script:Failed
    Assert-Fb2VoiceDeviceEvidence -Evidence $Evidence -EvidenceFilePath $EvidenceFilePath -RepoRoot $RepoRoot -MinFb2ApkVersion $MinFb2ApkVersion
    $caseFailures = $script:Failed - $before
    $script:Failed = $before
    $passedExpectation = if ($ShouldPass) { $caseFailures -eq 0 } else { $caseFailures -gt 0 }
    if ($passedExpectation) {
        Write-Output "OK`tself-test voice evidence $Name`tcase_failures=$caseFailures"
    } else {
        $script:SelfTestFailed += 1
        Write-Output "FAIL`tself-test voice evidence $Name`tcase_failures=$caseFailures shouldPass=$ShouldPass"
    }
}

function Invoke-AiCenterSelfTest {
    $script:SelfTestFailed = 0
    $tempRoot = [System.IO.Path]::GetTempPath()
    $tempDir = Join-Path $tempRoot ("fb2-ai-center-voice-evidence-selftest-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
    try {
        $logRef = "voice-logcat.txt"
        $screenRef = "voice-screen.png"
        Set-Content -Path (Join-Path $tempDir $logRef) -Value "SpeechRecognizer fallback self-test" -Encoding UTF8
        Set-Content -Path (Join-Path $tempDir $screenRef) -Value "fake screenshot bytes" -Encoding UTF8
        $evidencePath = Join-Path $tempDir "voice-evidence.json"
        $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
        $baseArtifacts = @(
            [pscustomobject]@{ type = "logcat"; ref = $logRef; note = "self-test logcat" },
            [pscustomobject]@{ type = "screenshot"; ref = $screenRef; note = "self-test screenshot" }
        )

        $valid = New-VoiceEvidenceSelfTestObject -Artifacts $baseArtifacts
        Invoke-VoiceEvidenceSelfTestCase "valid final-ready evidence" $valid $evidencePath $repoRoot $true

        $resolutionVariants = New-VoiceEvidenceSelfTestObject -Artifacts @(
            [pscustomobject]@{ type = "logcat"; ref = (Join-Path $tempDir $logRef); note = "absolute local logcat" },
            [pscustomobject]@{ type = "screenshot"; ref = $screenRef; note = "same-dir relative screenshot" },
            [pscustomobject]@{ type = "diagnostic_log"; ref = "docs/fb2-ai-center/README.md"; note = "repo-root relative diagnostic log" },
            [pscustomobject]@{ type = "video"; ref = "https://fb2.invalid/fb2-voice-evidence.mp4"; note = "remote URL accepted without fetch" }
        )
        Invoke-VoiceEvidenceSelfTestCase "accepts artifact resolution variants" $resolutionVariants $evidencePath $repoRoot $true

        $notReady = Copy-SelfTestObject $valid
        $notReady.finalAcceptanceReady = $false
        Invoke-VoiceEvidenceSelfTestCase "rejects finalAcceptanceReady false" $notReady $evidencePath $repoRoot $false

        $stringReady = Copy-SelfTestObject $valid
        $stringReady.finalAcceptanceReady = "true"
        Invoke-VoiceEvidenceSelfTestCase "rejects string finalAcceptanceReady" $stringReady $evidencePath $repoRoot $false

        $placeholder = Copy-SelfTestObject $valid
        $placeholder.artifacts[0].ref = "placeholder logcat path"
        Invoke-VoiceEvidenceSelfTestCase "rejects placeholder artifact ref" $placeholder $evidencePath $repoRoot $false

        $missingFile = Copy-SelfTestObject $valid
        $missingFile.artifacts[0].ref = "missing-logcat.txt"
        Invoke-VoiceEvidenceSelfTestCase "rejects missing local artifact" $missingFile $evidencePath $repoRoot $false

        $missingVisual = New-VoiceEvidenceSelfTestObject -Artifacts @([pscustomobject]@{ type = "logcat"; ref = $logRef; note = "self-test logcat" })
        Invoke-VoiceEvidenceSelfTestCase "rejects missing visual artifact" $missingVisual $evidencePath $repoRoot $false

        $missingLogcat = New-VoiceEvidenceSelfTestObject -Artifacts @([pscustomobject]@{ type = "screenshot"; ref = $screenRef; note = "self-test screenshot" })
        Invoke-VoiceEvidenceSelfTestCase "rejects missing logcat artifact" $missingLogcat $evidencePath $repoRoot $false

        $emptyArtifacts = New-VoiceEvidenceSelfTestObject -Artifacts @()
        Invoke-VoiceEvidenceSelfTestCase "rejects empty artifact list" $emptyArtifacts $evidencePath $repoRoot $false

        $blankArtifactType = New-VoiceEvidenceSelfTestObject -Artifacts @(
            [pscustomobject]@{ type = ""; ref = $logRef; note = "blank type" },
            [pscustomobject]@{ type = "screenshot"; ref = $screenRef; note = "self-test screenshot" }
        )
        Invoke-VoiceEvidenceSelfTestCase "rejects blank artifact type" $blankArtifactType $evidencePath $repoRoot $false

        $blankArtifactRef = New-VoiceEvidenceSelfTestObject -Artifacts @(
            [pscustomobject]@{ type = "logcat"; ref = ""; note = "blank ref" },
            [pscustomobject]@{ type = "screenshot"; ref = $screenRef; note = "self-test screenshot" }
        )
        Invoke-VoiceEvidenceSelfTestCase "rejects blank artifact ref" $blankArtifactRef $evidencePath $repoRoot $false

        $lowApk = Copy-SelfTestObject $valid
        $lowApk.apk.versionName = "1.1.47"
        Invoke-VoiceEvidenceSelfTestCase "rejects low APK version" $lowApk $evidencePath $repoRoot $false

        $missingSystemAsr = Copy-SelfTestObject $valid
        $missingSystemAsr.checks.systemAsrSuccess = $false
        Invoke-VoiceEvidenceSelfTestCase "rejects missing system ASR success" $missingSystemAsr $evidencePath $repoRoot $false

        Invoke-Fb2ContextProjectionSelfTests
    } finally {
        $resolvedTemp = (Resolve-Path -LiteralPath $tempDir -ErrorAction SilentlyContinue)
        if ($resolvedTemp -and $resolvedTemp.Path.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolvedTemp.Path -Recurse -Force
        }
    }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$script:SelfTestFailed"
    if ($script:SelfTestFailed -gt 0) {
        exit 1
    }
    exit 0
}

if ($SelfTest) {
    Invoke-AiCenterSelfTest
}

function Resolve-MainTokenFromFb2 {
    if ($MainToken) {
        return [pscustomobject]@{
            Token = $MainToken.Trim()
            Source = "main-token"
            Fb2UserId = $ExternalUserId
        }
    }

    if (-not $Fb2UserToken) {
        if (-not $Fb2Username -or -not $Fb2Password) {
            return $null
        }
        $login = Invoke-Json -Url "$Fb2Base/api/auth/login" -Method "POST" -Body @{
            username = $Fb2Username
            password = $Fb2Password
        }
        if (-not $login.success -or -not $login.data.token.access_token) {
            throw "fb2 login failed"
        }
        $Fb2UserToken = [string]$login.data.token.access_token
        $script:Fb2LoginUserId = [string]$login.data.user.id
    }

    $fb2UserHeaders = @{ Authorization = "Bearer $($Fb2UserToken.Trim())" }
    $session = Invoke-Json -Url "$Fb2Base/api/main-project/session" -Headers $fb2UserHeaders -Method "POST" -Body @{
        deviceName = "main-ai-center-smoke"
    }
    if (-not $session.success -or -not $session.data.token) {
        throw "fb2 main-project session bridge failed"
    }

    return [pscustomobject]@{
        Token = [string]$session.data.token
        Source = "fb2-session-bridge"
        Fb2UserId = [string]$script:Fb2LoginUserId
    }
}

try {
    $resolvedMainToken = Resolve-MainTokenFromFb2
    if ($resolvedMainToken) {
        $MainToken = [string]$resolvedMainToken.Token
        Write-Output "OK`tmain token resolved`t$($resolvedMainToken.Source)"
        if (-not $ExternalUserId -and $resolvedMainToken.Fb2UserId) {
            $ExternalUserId = [string]$resolvedMainToken.Fb2UserId
            Write-Output "OK`tfb2 external user`t$ExternalUserId"
        }
    }
} catch {
    Fail "main token resolved" $_.Exception.Message
}

Write-Output "== Main project contract =="

try {
    $health = (Invoke-WebRequest -Uri "$MainBase/health" -UseBasicParsing -TimeoutSec 10).Content.Trim()
    Assert-True ($health -eq "OK") "main health" $health
} catch {
    Fail "main health" $_.Exception.Message
}

try {
    $version = Invoke-Json "$MainBase/api/server/version"
    Assert-True ([bool]$version.gitSha) "main version" "$($version.versionName) $($version.gitSha)"
} catch {
    Fail "main version" $_.Exception.Message
}

if ($MainToken) {
    try {
        $mainHeaders = @{ "Authorization" = "Bearer $($MainToken.Trim())" }
        $bootstrap = Invoke-Json -Url "$MainBase/api/external/apps/fb2/chat-bootstrap" -Headers $mainHeaders
        Assert-True ($bootstrap.aiReply.schema -eq "external_app.ai_reply.v1") "chat-bootstrap aiReply"
        Assert-True ([bool]$bootstrap.billing) "chat-bootstrap billing"
        Assert-True ($bootstrap.experience.usagePolicy.aiReplyGeneration -eq "billable") "chat-bootstrap experience AI billable"
        Assert-True ($bootstrap.billing.gates.beforeAiReplyGeneration -eq "check_balance_or_trial_credit") "chat-bootstrap before AI reply gate"
        Assert-ContainsValue $bootstrap.aiReply.freePreparationSteps "external_context_fetch" "chat-bootstrap AI reply keeps context fetch free"
        if ($SkipVoiceContractChecks) {
            Pass "data-only acceptance excludes voice contract" "ASR/TTS deferred by current task scope"
        } else {
            Assert-True ([bool]$bootstrap.voice.composer) "chat-bootstrap voice composer"
            Assert-True ($bootstrap.voice.androidSdk.module -eq "android/chat-voice-kit") "chat-bootstrap voice sdk module" "$($bootstrap.voice.androidSdk.module)"
            Assert-ContainsValue $bootstrap.voice.androidSdk.publicComponents "VoiceComposerView" "chat-bootstrap voice sdk exposes VoiceComposerView"
            Assert-ContainsValue $bootstrap.voice.androidSdk.publicComponents "VoiceComposerBootstrap" "chat-bootstrap voice sdk exposes VoiceComposerBootstrap"
            Assert-ContainsValue $bootstrap.voice.androidSdk.publicComponents "ChatVoiceEventSink" "chat-bootstrap voice sdk exposes events"
            Assert-True ($bootstrap.voice.composer.requiredForMainProjectLikeExperience -eq $true) "chat-bootstrap requires voice composer"
            Assert-True ($bootstrap.voice.composer.recommendedConfigApi -eq "VoiceComposerBootstrap.applyFb2GroupChatConfig(...)") "chat-bootstrap recommended voice config" "$($bootstrap.voice.composer.recommendedConfigApi)"
            Assert-True ($bootstrap.voice.composer.defaultConfig.recordingOverlayEnabled -eq $true) "chat-bootstrap recording overlay enabled"
            Assert-True ($bootstrap.voice.composer.defaultConfig.asr.serverFallbackEnabled -eq $true) "chat-bootstrap server ASR fallback enabled"
            Assert-True ($bootstrap.voice.composer.defaultConfig.asr.serverConfigRequired -eq $true) "chat-bootstrap server ASR config required"
            Assert-True ([int]$bootstrap.voice.composer.defaultConfig.asr.localResultTimeoutMs -gt 0) "chat-bootstrap local ASR result timeout" "$($bootstrap.voice.composer.defaultConfig.asr.localResultTimeoutMs)ms"
            Assert-True ($bootstrap.voice.composer.defaultConfig.asr.localEngineFallbackEnabled -eq $true) "chat-bootstrap local engine fallback enabled"
            Assert-True ($bootstrap.voice.composer.defaultConfig.asr.prewarmLocalEngine -eq $true) "chat-bootstrap local ASR prewarm enabled"
            Assert-ContainsValue $bootstrap.voice.composer.states "SERVER_PROCESSING" "chat-bootstrap exposes server processing state"
            Assert-ContainsValue $bootstrap.voice.composer.zones "AI_REPLY" "chat-bootstrap exposes AI reply zone"
            Assert-ContainsValue $bootstrap.voice.composer.callbacks "onVoiceServerFallbackStarted" "chat-bootstrap exposes fallback callback"
            Assert-True ($bootstrap.voice.asr.localFirst -eq $true) "chat-bootstrap ASR local first"
            Assert-True ($bootstrap.voice.asr.serverFallback -eq $true) "chat-bootstrap ASR server fallback"
            Assert-True ($bootstrap.voice.asr.uploadEndpoint -eq "/api/voice/asr") "chat-bootstrap ASR endpoint" "$($bootstrap.voice.asr.uploadEndpoint)"
            Assert-True ($bootstrap.voice.asr.billing -eq "free_auth_and_limits_only") "chat-bootstrap ASR free billing"
            Assert-True ($bootstrap.voice.tts.billing -eq "free_auth_and_limits_only") "chat-bootstrap TTS free billing"
            Assert-True ($bootstrap.experience.usagePolicy.asr -eq "free") "chat-bootstrap experience ASR free"
            Assert-True ($bootstrap.experience.usagePolicy.tts -eq "free") "chat-bootstrap experience TTS free"
            Assert-True ($bootstrap.experience.controls.fullWidthHoldToTalkButton -eq $true) "chat-bootstrap hold-to-talk full-width"
            Assert-True ($bootstrap.billing.gates.beforeAsr -eq "never_check_ai_balance") "chat-bootstrap before ASR gate"
            Assert-True ($bootstrap.billing.gates.beforeTts -eq "never_check_ai_balance") "chat-bootstrap before TTS gate"
            Assert-ContainsValue $bootstrap.aiReply.freePreparationSteps "asr" "chat-bootstrap AI reply keeps ASR free"
            Assert-ContainsValue $bootstrap.aiReply.freePreparationSteps "tts" "chat-bootstrap AI reply keeps TTS free"
        }
    } catch {
        Fail "chat-bootstrap" $_.Exception.Message
    }
} else {
    Skip "chat-bootstrap" "set ELON_MAIN_TOKEN or -MainToken to verify authenticated bootstrap"
}

$requiredFb2ToolIds = @(
    "context_pack",
    "today_matches",
    "match_analysis_brief",
    "group_opinion_summary",
    "search_matches",
    "search_user_orders",
    "user_orders",
    "platform_orders",
    "record_context_feedback",
    "list_context_feedbacks",
    "context_feedback_summary",
    "context_audit_summary",
    "tool_manifest"
)
$liveToolIds = @()

try {
    $contract = Invoke-Json "$MainBase/api/external/apps/fb2/context-contract"
    $policy = $contract.live_tool_manifest.main_project_tool_execution_policy
    $liveToolIds = @($contract.live_tool_manifest.tool_ids)
    $answerPolicy = $contract.answer_policy_contract
    $projectionContract = $contract.domain_context_projection_contract
    $projectionSections = @($projectionContract.required_sections)
    $projectionSectionIds = @($projectionSections | ForEach-Object { $_.id })
    $projectionSourceKinds = @($projectionContract.source_registry.required_kinds)
    $projectionAntiPatterns = @($projectionContract.anti_patterns)
    $projectionRetrievalFields = @($projectionContract.retrieval_projection.recommended_fields)
    $projectionPermissions = @($projectionContract.permission_projection)
    $projectionQualityRoutes = @($projectionContract.quality_closure.required_feedback_routes)
    $projectionReadiness = $projectionContract.quality_closure.minimum_non_synthetic_ready
    $evalScenarios = @($answerPolicy.eval_scenarios)
    $evalScenarioIds = @($evalScenarios | ForEach-Object { $_.id })
    Assert-True ($contract.live_tool_manifest.status -eq "ready") "live manifest ready" "tool_count=$($contract.live_tool_manifest.tool_count)"
    Assert-True ($policy.schema -eq "external_app.live_tool_execution_policy.v1") "live manifest execution policy"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "search_matches") -and ($policy.chat_auto_executable_tool_ids -contains "search_group_opinions")) "auto executable core tools"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "match_analysis_brief") -and ($policy.chat_auto_executable_tool_ids -contains "group_opinion_summary")) "auto executable aggregate tools"
    Assert-True ($policy.manifest_only_tool_ids -contains "record_context_feedback") "callback tool is not chat-auto-executable"
    Assert-True (@($policy.main_project_allowed_missing_tool_ids).Count -eq 0) "no allowed tool missing in live fb2 manifest"
    foreach ($toolId in $requiredFb2ToolIds) {
        Assert-ContainsValue $liveToolIds $toolId "live manifest required tool: $toolId"
    }
    Assert-True ($answerPolicy.schema -eq "fb2.answer_policy.v1") "answer policy schema"
    Assert-True (@($answerPolicy.canonical_eval_questions).Count -ge 6) "answer policy canonical eval questions" "count=$(@($answerPolicy.canonical_eval_questions).Count)"
    Assert-True (($evalScenarioIds -contains "today_matches_analysis") -and ($evalScenarioIds -contains "my_ticket_analysis")) "answer policy core eval scenarios"
    Assert-True (($evalScenarioIds -contains "platform_order_risk") -and ($evalScenarioIds -contains "group_opinion_summary")) "answer policy aggregate eval scenarios"
    Assert-True (($evalScenarioIds -contains "selected_message_review") -and ($evalScenarioIds -contains "source_reference_audit")) "answer policy audit eval scenarios"

    $todayScenario = Find-EvalScenario $evalScenarios "today_matches_analysis"
    Assert-ScenarioContains $todayScenario "required_source_kinds" @("match", "odds") "eval scenario today source kinds"
    Assert-ScenarioContains $todayScenario "required_citations" @("match_id", "context_audit_id") "eval scenario today citations"
    Assert-ScenarioContains $todayScenario "forbidden_outputs" @("guaranteed_win", "fabricated_odds") "eval scenario today forbidden outputs"

    $ticketScenario = Find-EvalScenario $evalScenarios "my_ticket_analysis"
    Assert-True ($ticketScenario.permission_boundary -eq "current_user_only") "eval scenario my ticket permission" "$($ticketScenario.permission_boundary)"
    Assert-ScenarioContains $ticketScenario "required_headers" @("X-FB2-AI-CONTEXT-USER-ID") "eval scenario my ticket headers"
    Assert-ScenarioContains $ticketScenario "required_query_fields" @("external_user_id") "eval scenario my ticket query fields"
    Assert-ScenarioContains $ticketScenario "required_citations" @("order_id", "match_id", "context_audit_id") "eval scenario my ticket citations"
    Assert-ScenarioContains $ticketScenario "forbidden_outputs" @("other_user_order_detail", "guaranteed_win") "eval scenario my ticket forbidden outputs"

    $platformScenario = Find-EvalScenario $evalScenarios "platform_order_risk"
    Assert-True ($platformScenario.permission_boundary -eq "anonymous_aggregate_only") "eval scenario platform permission" "$($platformScenario.permission_boundary)"
    Assert-ScenarioContains $platformScenario "required_headers" @("X-FB2-AI-CONTEXT-SCOPE=platform_order_summary") "eval scenario platform headers"
    Assert-ScenarioContains $platformScenario "required_query_fields" @("include_platform_orders=true") "eval scenario platform query fields"
    Assert-ScenarioContains $platformScenario "required_citations" @("platform_order_summary", "context_audit_id") "eval scenario platform citations"
    Assert-ScenarioContains $platformScenario "forbidden_outputs" @("single_user_order_detail", "user_identity_leak") "eval scenario platform forbidden outputs"

    $opinionScenario = Find-EvalScenario $evalScenarios "group_opinion_summary"
    Assert-ScenarioContains $opinionScenario "required_source_kinds" @("group_message", "opinion_memory") "eval scenario opinion source kinds"
    Assert-ScenarioContains $opinionScenario "required_citations" @("message_id", "context_audit_id") "eval scenario opinion citations"
    Assert-ScenarioContains $opinionScenario "forbidden_outputs" @("group_opinion_as_fact", "fabricated_group_view") "eval scenario opinion forbidden outputs"

    $selectedScenario = Find-EvalScenario $evalScenarios "selected_message_review"
    Assert-ScenarioContains $selectedScenario "entrypoints" @("selected_message_ai_reply") "eval scenario selected entrypoint"
    Assert-ScenarioContains $selectedScenario "required_query_fields" @("selected_message_id", "topic_hint") "eval scenario selected query fields"
    Assert-ScenarioContains $selectedScenario "required_citations" @("selected_message_id", "match_id", "context_audit_id") "eval scenario selected citations"
    Assert-ScenarioContains $selectedScenario "forbidden_outputs" @("unsupported_claim_verdict", "guaranteed_win") "eval scenario selected forbidden outputs"

    $auditScenario = Find-EvalScenario $evalScenarios "source_reference_audit"
    Assert-ScenarioContains $auditScenario "required_source_kinds" @("citation_sources") "eval scenario source audit source kinds"
    Assert-ScenarioContains $auditScenario "required_citations" @("context_audit_id") "eval scenario source audit citations"
    Assert-ScenarioContains $auditScenario "forbidden_outputs" @("uncited_claim", "invented_source_id") "eval scenario source audit forbidden outputs"

    Assert-True ($projectionContract.schema -eq "fb2.domain_context_projection.v1") "domain projection schema" "$($projectionContract.schema)"
    Assert-True ($projectionContract.format.wrapper -eq "fb2_context_pack") "domain projection wrapper" "$($projectionContract.format.wrapper)"
    Assert-ContainsValue $projectionSectionIds "match_facts" "domain projection section: match facts"
    Assert-ContainsValue $projectionSectionIds "user_order_slice" "domain projection section: user orders"
    Assert-ContainsValue $projectionSectionIds "platform_order_summary" "domain projection section: platform summary"
    Assert-ContainsValue $projectionSectionIds "group_opinion_slice" "domain projection section: group opinions"
    Assert-ContainsValue $projectionSectionIds "retrieval_evidence" "domain projection section: retrieval evidence"
    Assert-ContainsValue $projectionSectionIds "quality_feedback" "domain projection section: quality feedback"
    Assert-ContainsValue $projectionSourceKinds "match" "domain projection source kind: match"
    Assert-ContainsValue $projectionSourceKinds "odds" "domain projection source kind: odds"
    Assert-ContainsValue $projectionSourceKinds "user_order" "domain projection source kind: user order"
    Assert-ContainsValue $projectionSourceKinds "group_message" "domain projection source kind: group message"
    Assert-ContainsValue $projectionSourceKinds "opinion_memory" "domain projection source kind: opinion memory"
    Assert-ContainsValue $projectionSourceKinds "platform_order_summary" "domain projection source kind: platform summary"
    Assert-ContainsValue $projectionSourceKinds "feedback" "domain projection source kind: feedback"
    Assert-ContainsValue $projectionSourceKinds "opinion_adoption" "domain projection source kind: opinion adoption"
    Assert-ContainsValue $projectionAntiPatterns "raw_embedding_dump" "domain projection anti-pattern: raw embedding dump"
    Assert-ContainsValue $projectionAntiPatterns "platform_order_detail_leak" "domain projection anti-pattern: platform order leak"
    Assert-ContainsValue $projectionRetrievalFields "topic_hint" "domain projection retrieval field: topic hint"
    Assert-ContainsValue $projectionRetrievalFields "match_reason" "domain projection retrieval field: match reason"
    Assert-ContainsValue $projectionRetrievalFields "permission_scope" "domain projection retrieval field: permission scope"
    Assert-ContainsValue $projectionRetrievalFields "truncated" "domain projection retrieval field: truncated"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/feedback" "domain projection quality route: feedback"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/feedback-summary" "domain projection quality route: feedback summary"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/opinion-adoption-summary" "domain projection quality route: opinion adoption summary"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/quality-summary" "domain projection quality route: quality summary"
    Assert-True ([int]$projectionReadiness.feedback_count -ge 1) "domain projection readiness: feedback count" "feedback_count=$($projectionReadiness.feedback_count)"
    Assert-True ([int]$projectionReadiness.opinion_adoption_count -ge 1) "domain projection readiness: opinion adoption count" "opinion_adoption_count=$($projectionReadiness.opinion_adoption_count)"
    Assert-True ([string]$projectionReadiness.opinion_memory_ref_count -eq "present") "domain projection readiness: opinion memory refs" "opinion_memory_ref_count=$($projectionReadiness.opinion_memory_ref_count)"

    $userOrderPermission = Find-ProjectionPermission $projectionPermissions "user_orders"
    Assert-True ($null -ne $userOrderPermission) "domain projection permission: user orders present"
    Assert-True ($userOrderPermission.scope -eq "current_user_only") "domain projection permission: user orders scope" "$($userOrderPermission.scope)"
    Assert-ScenarioContains $userOrderPermission "required_request" @("external_user_id", "X-FB2-AI-CONTEXT-USER-ID") "domain projection permission: user orders required request"
    Assert-ScenarioContains $userOrderPermission "forbidden" @("other_user_order_detail", "raw_user_identity") "domain projection permission: user orders forbidden"

    $platformPermission = Find-ProjectionPermission $projectionPermissions "platform_order_summary"
    Assert-True ($null -ne $platformPermission) "domain projection permission: platform summary present"
    Assert-True ($platformPermission.scope -eq "anonymous_aggregate_only") "domain projection permission: platform summary scope" "$($platformPermission.scope)"
    Assert-ScenarioContains $platformPermission "required_request" @("include_platform_orders=true", "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary") "domain projection permission: platform summary required request"
    Assert-ScenarioContains $platformPermission "forbidden" @("single_user_order_detail", "raw_user_identity") "domain projection permission: platform summary forbidden"

    $groupOpinionPermission = Find-ProjectionPermission $projectionPermissions "group_opinions"
    Assert-True ($null -ne $groupOpinionPermission) "domain projection permission: group opinions present"
    Assert-True ($groupOpinionPermission.scope -eq "group_visible") "domain projection permission: group opinions scope" "$($groupOpinionPermission.scope)"
    Assert-ScenarioContains $groupOpinionPermission "required_request" @("group_id") "domain projection permission: group opinions required request"
    Assert-ScenarioContains $groupOpinionPermission "forbidden" @("private_message", "opinion_without_message_id") "domain projection permission: group opinions forbidden"
} catch {
    Fail "context-contract" $_.Exception.Message
}

Write-Output ""
Write-Output "== fb2 dynamic discovery =="
try {
    $integration = Invoke-Json "$Fb2Base/api/main-project/integration"
    $integrationData = $integration.data
    Assert-True ($integration.success -eq $true) "fb2 integration discovery" "project_id=$($integrationData.project_id)"
    Assert-True ($integrationData.configured -eq $true) "fb2 integration configured" "configured=$($integrationData.configured)"
    Assert-True ($integrationData.routing_mode -eq "main_project_ready") "fb2 integration routing mode" "routing_mode=$($integrationData.routing_mode)"
    Assert-True ($integrationData.service_token_header -eq "X-FB2-AI-CENTER-TOKEN") "fb2 integration token header" "$($integrationData.service_token_header)"

    $integrationEndpointNames = Get-ObjectPropertyNames $integrationData.fb2_context_endpoints
    foreach ($endpointName in @(
        "context_readiness",
        "context_pack",
        "tool_manifest",
        "match_analysis_brief",
        "group_opinion_summary",
        "user_orders",
        "platform_orders",
        "context_quality_summary",
        "context_permission_summary"
    )) {
        Assert-ContainsValue $integrationEndpointNames $endpointName "fb2 integration endpoint: $endpointName"
    }

    $groupIds = @($integrationData.group_mappings | ForEach-Object { $_.local_group_id })
    Assert-ContainsValue $groupIds $GroupId "fb2 integration group mapping"
} catch {
    Fail "fb2 integration discovery" $_.Exception.Message
}

if (-not $Fb2Token) {
    try {
        $readinessStatus = Invoke-HttpStatus -Url "$Fb2Base/api/main-project/context/readiness"
        Assert-StatusCode $readinessStatus 401 "fb2 readiness requires service token"
        $manifestStatus = Invoke-HttpStatus -Url "$Fb2Base/api/main-project/context/tool-manifest"
        Assert-StatusCode $manifestStatus 401 "fb2 tool manifest requires service token"
    } catch {
        Fail "fb2 protected dynamic discovery" $_.Exception.Message
    }
} else {
    try {
        $fb2DiscoveryHeaders = Fb2-Headers
        $readiness = Invoke-Json -Url "$Fb2Base/api/main-project/context/readiness" -Headers $fb2DiscoveryHeaders
        Assert-True ($readiness.success -eq $true) "fb2 authenticated readiness" "success=$($readiness.success)"
        $readinessValue = @(
            $readiness.data.status,
            $readiness.data.readiness_status,
            $readiness.data.context_status,
            $readiness.data.context_readiness.status
        ) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -First 1
        Assert-True ([bool]$readinessValue) "fb2 authenticated readiness status" "status=$readinessValue"
        Assert-ContainsValue @("ready", "degraded", "blocked", "unavailable") $readinessValue "fb2 authenticated readiness status value"

        $directManifest = Invoke-Json -Url "$Fb2Base/api/main-project/context/tool-manifest" -Headers $fb2DiscoveryHeaders
        Assert-True ($directManifest.success -eq $true) "fb2 authenticated tool manifest" "success=$($directManifest.success)"
        $directToolIds = Get-Fb2ManifestToolIds $directManifest
        Assert-MinCount $directToolIds 1 "fb2 authenticated manifest tool ids"
        foreach ($toolId in $requiredFb2ToolIds) {
            Assert-ContainsValue $directToolIds $toolId "fb2 authenticated manifest required tool: $toolId"
        }
        foreach ($toolId in $liveToolIds) {
            Assert-ContainsValue $directToolIds $toolId "fb2 direct manifest matches main contract: $toolId"
        }
    } catch {
        Fail "fb2 authenticated dynamic discovery" $_.Exception.Message
    }
}

if ($CheckFb2ApkVersion) {
    Write-Output ""
    Write-Output "== fb2 APK release =="

    try {
        $appVersion = Invoke-Json "$Fb2Base/api/app-version"
        $versionText = [string]$appVersion.version
        $apkUrl = Resolve-AbsoluteUrl ([string]$appVersion.apk_url)
        Assert-True ([bool]$versionText) "fb2 APK version present" $versionText
        Assert-True ((Compare-VersionParts $versionText $MinFb2ApkVersion) -ge 0) "fb2 APK minimum version" "version=$versionText min=$MinFb2ApkVersion"
        Assert-True ([string]$appVersion.update_kind -eq $ExpectedFb2UpdateKind) "fb2 APK update kind" "$($appVersion.update_kind)"
        Assert-True ([string]$appVersion.checksum -like "sha256:*") "fb2 APK checksum" "$($appVersion.checksum)"
        Assert-True ([int64]$appVersion.size -gt 0) "fb2 APK size" "$($appVersion.size)"
        Assert-True ([bool]$apkUrl) "fb2 APK url" $apkUrl

        $head = Invoke-WebRequest -UseBasicParsing -Uri $apkUrl -Method Head -TimeoutSec $RequestTimeoutSec
        $contentType = [string]$head.Headers["Content-Type"]
        $contentDisposition = [string]$head.Headers["Content-Disposition"]
        Assert-True (($contentType -like "*android.package-archive*") -or ($contentDisposition -like "*.apk*")) "fb2 APK download head" "contentType=$contentType disposition=$contentDisposition"
    } catch {
        Fail "fb2 APK release" $_.Exception.Message
    }
}

if ($CheckLocalVoiceSdkBuild) {
    Write-Output ""
    Write-Output "== Local Android voice SDK build =="

    $androidDir = Join-Path $PSScriptRoot "..\android"
    $gradleBat = Join-Path $androidDir "gradlew.bat"
    if (-not (Test-Path $gradleBat)) {
        Fail "local voice SDK build" "gradlew.bat not found: $gradleBat"
    } else {
        Push-Location $androidDir
        try {
            & $gradleBat $VoiceSdkGradleTask --quiet
            Assert-True ($LASTEXITCODE -eq 0) "local voice SDK build" $VoiceSdkGradleTask
        } catch {
            Fail "local voice SDK build" $_.Exception.Message
        } finally {
            Pop-Location
        }
    }
}

if ($RequireVoiceDeviceEvidence) {
    Write-Output ""
    Write-Output "== fb2 voice device evidence =="

    if (-not $VoiceDeviceEvidencePath) {
        Fail "voice device evidence" "Pass -VoiceDeviceEvidencePath with fb2.voice_device_evidence.v1 JSON"
    } elseif (-not (Test-Path $VoiceDeviceEvidencePath)) {
        Fail "voice device evidence" "file not found: $VoiceDeviceEvidencePath"
    } else {
        try {
            $evidence = Get-Content -Raw -Path $VoiceDeviceEvidencePath | ConvertFrom-Json
            $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
            Assert-Fb2VoiceDeviceEvidence -Evidence $evidence -EvidenceFilePath $VoiceDeviceEvidencePath -RepoRoot $repoRoot -MinFb2ApkVersion $MinFb2ApkVersion
        } catch {
            Fail "voice device evidence" $_.Exception.Message
        }
    }
}

if (-not $Fb2Token) {
    if ($RequireFb2Live -or $RequireAllScenarios -or $qualityCheckRequested -or $permissionCheckRequested) {
        Fail "fb2 live token" "FB2_AI_CENTER_TOKEN or -Fb2Token is required"
    } else {
        Skip "fb2 live data" "set FB2_AI_CENTER_TOKEN to verify Context Pack scenarios"
    }
} else {
    Write-Output ""
    Write-Output "== fb2 live data scenarios =="
    $fb2Headers = Fb2-Headers

    try {
        $manifest = Invoke-Json "$Fb2Base/api/main-project/context/tool-manifest" $fb2Headers
        Assert-True ($manifest.success -eq $true) "fb2 tool manifest"
        Assert-True (@($manifest.data.tool_contract.endpoints).Count -gt 0) "fb2 manifest endpoint count" "count=$(@($manifest.data.tool_contract.endpoints).Count)"
    } catch {
        Fail "fb2 tool manifest" $_.Exception.Message
    }

    try {
        $topic = Encode-QueryValue "今天比赛怎么看"
        $url = "$Fb2Base/api/main-project/context/pack?group_id=$GroupId$($amp)topic_hint=$topic$($amp)limit=10$($amp)discussion_limit=20"
        $pack = Invoke-Json -Url $url -Headers $fb2Headers
        Assert-True ($pack.success -eq $true) "scenario: today matches context pack"
        Assert-True ([bool]$pack.data.context_audit_id) "scenario: today matches context audit" "$($pack.data.context_audit_id)"
        Assert-True ([bool]$pack.data.context_pack) "scenario: today matches context body"
        Assert-True ($null -ne $pack.data.citation_sources) "scenario: today matches citation sources field"
        if ($CheckDomainProjection) {
            Assert-Fb2ContextPackProjection -Data $pack.data -Scenario "today matches context pack" -ExpectedSourceKinds @("match", "odds", "context_audit")
        }
        if ($RequireAllScenarios) {
            Assert-MinCount $pack.data.matches 1 "scenario: today matches has match data"
            Assert-MinCount $pack.data.citation_sources 1 "scenario: today matches has citation sources"
        }
    } catch {
        Fail "scenario: today matches" $_.Exception.Message
    }

    try {
        $topic = Encode-QueryValue "今天比赛怎么看"
        $url = "$Fb2Base/api/main-project/context/match-analysis-brief?group_id=$GroupId$($amp)topic_hint=$topic$($amp)limit=6"
        $brief = Invoke-Json -Url $url -Headers $fb2Headers
        Assert-True ($brief.success -eq $true) "scenario: match analysis brief"
        Assert-True ($null -ne $brief.data.matches) "scenario: match analysis has matches field"
        Assert-True ($null -ne $brief.data.usage_policy) "scenario: match analysis usage policy"
        if ($RequireAllScenarios) {
            Assert-MinCount $brief.data.matches 1 "scenario: match analysis has match data"
        }
    } catch {
        Fail "scenario: match analysis brief" $_.Exception.Message
    }

    try {
        $query = Encode-QueryValue "群里大家怎么看这场"
        $url = "$Fb2Base/api/main-project/context/group-opinion-summary?group_id=$GroupId$($amp)query=$query$($amp)limit=80"
        $opinions = Invoke-Json -Url $url -Headers $fb2Headers
        Assert-True ($opinions.success -eq $true) "scenario: group opinions summary"
        Assert-True ($null -ne $opinions.data.opinion_summary) "scenario: group opinions summary field"
        Assert-True ($null -ne $opinions.data.usage_policy) "scenario: group opinions usage policy"
        if ($RequireAllScenarios) {
            Assert-MinCount $opinions.data.opinion_summary 1 "scenario: group opinions has summary data"
        }
    } catch {
        Fail "scenario: group opinions summary" $_.Exception.Message
    }

    try {
        $reviews = Invoke-Json "$Fb2Base/api/main-project/context/opinion-result-review-summary?group_id=$GroupId" $fb2Headers
        Assert-True ($reviews.success -eq $true) "scenario: message correctness review summary"
        Assert-True ($null -ne $reviews.data.summary) "scenario: result review summary field"
        Assert-True ($null -ne $reviews.data.usage_policy) "scenario: result review usage policy"
    } catch {
        Fail "scenario: message correctness review summary" $_.Exception.Message
    }

    try {
        $summaryTool = Invoke-Json -Url "$Fb2Base/api/main-project/tools/execute" -Headers $fb2Headers -Method "POST" -Body @{
            request_id = "main-smoke-group-opinion-summary"
            tool_name = "group_opinion_summary"
            group_id = $GroupId
            arguments = @{
                query = "群里大家怎么看这场"
                limit = 5
            }
            reason = "main smoke"
        }
        Assert-True ($summaryTool.success -eq $true) "tool execute: group_opinion_summary"
        Assert-True ($summaryTool.visibility -eq "single_group_lightweight_memory") "tool execute: group_opinion_summary visibility"

        $briefTool = Invoke-Json -Url "$Fb2Base/api/main-project/tools/execute" -Headers $fb2Headers -Method "POST" -Body @{
            request_id = "main-smoke-match-analysis-brief"
            tool_name = "match_analysis_brief"
            group_id = $GroupId
            arguments = @{
                topic_hint = "今天比赛怎么看"
                limit = 5
                order_limit = 1
            }
            reason = "main smoke"
        }
        Assert-True ($briefTool.success -eq $true) "tool execute: match_analysis_brief"
        Assert-True ($briefTool.visibility -eq "match_focused_brief") "tool execute: match_analysis_brief visibility"
    } catch {
        Fail "tool execute aggregate tools" $_.Exception.Message
    }

    if ($ExternalUserId) {
        try {
            $topic = Encode-QueryValue "帮我分析我的票"
            $userHeaders = Fb2-Headers -UserId $ExternalUserId
            $url = "$Fb2Base/api/main-project/context/pack?group_id=$GroupId$($amp)external_user_id=$ExternalUserId$($amp)topic_hint=$topic$($amp)limit=10$($amp)order_limit=10"
            $orders = Invoke-Json -Url $url -Headers $userHeaders
            Assert-True ($orders.success -eq $true) "scenario: my ticket context pack"
            Assert-True ([bool]$orders.data.context_audit_id) "scenario: my ticket context audit" "$($orders.data.context_audit_id)"
            Assert-True ($null -ne $orders.data.user_orders) "scenario: my ticket user_orders field"
            if ($CheckDomainProjection) {
                Assert-Fb2ContextPackProjection -Data $orders.data -Scenario "my ticket context pack" -ExpectedSourceKinds @("user_order", "ticket", "context_audit")
            }
            if ($RequireAllScenarios) {
                Assert-MinCount $orders.data.user_orders 1 "scenario: my ticket has user orders"
                Assert-MinCount $orders.data.citation_sources 1 "scenario: my ticket has citation sources"
            }
        } catch {
            Fail "scenario: my ticket" $_.Exception.Message
        }
    } else {
        if ($RequireAllScenarios) {
            Fail "scenario: my ticket" "-ExternalUserId is required"
        } else {
            Skip "scenario: my ticket" "pass -ExternalUserId to verify current-user order context"
        }
    }

    if ($IncludePlatformOrderSummary) {
        try {
            $platformHeaders = Fb2-Headers -PlatformScope $true
            $platform = Invoke-Json "$Fb2Base/api/main-project/context/platform-orders" $platformHeaders
            Assert-True ($platform.success -eq $true) "scenario: platform order risk"
            Assert-True ($null -ne $platform.data.summary) "scenario: platform order summary field"
            Assert-True ($null -ne $platform.data.usage_policy) "scenario: platform order usage policy"
            if ($RequireAllScenarios) {
                Assert-MinCount $platform.data.summary 1 "scenario: platform order has summary data"
            }
        } catch {
            Fail "scenario: platform order risk" $_.Exception.Message
        }
    } else {
        if ($RequireAllScenarios) {
            Fail "scenario: platform order risk" "-IncludePlatformOrderSummary is required"
        } else {
            Skip "scenario: platform order risk" "pass -IncludePlatformOrderSummary to verify privileged aggregate context"
        }
    }

    if ($permissionCheckRequested) {
        Write-Output ""
        Write-Output "== fb2 permission boundaries =="

        if (-not $ExternalUserId) {
            Fail "permission boundaries" "-ExternalUserId is required"
        } else {
            $permissionSince = (Get-Date).ToUniversalTime().AddSeconds(-5).ToString("o")
            try {
                $topic = Encode-QueryValue "帮我分析我的票"
                $url = "$Fb2Base/api/main-project/context/pack?group_id=$GroupId$($amp)external_user_id=$ExternalUserId$($amp)topic_hint=$topic$($amp)limit=3$($amp)order_limit=1"
                $missingUserHeader = Invoke-HttpStatus -Url $url -Headers $fb2Headers
                Assert-StatusCode $missingUserHeader 403 "permission: context pack requires current user header"

                $mismatchedUserId = Get-MismatchedFb2UserId $ExternalUserId
                $mismatchedUserHeaders = Fb2-Headers -UserId $mismatchedUserId
                $mismatchedUserHeader = Invoke-HttpStatus -Url $url -Headers $mismatchedUserHeaders
                Assert-StatusCode $mismatchedUserHeader 403 "permission: context pack rejects mismatched current user header"

                $platformWithoutScope = Invoke-HttpStatus -Url "$Fb2Base/api/main-project/context/platform-orders" -Headers $fb2Headers
                Assert-StatusCode $platformWithoutScope 403 "permission: platform summary requires scope"

                $userToolWithoutHeader = Invoke-HttpStatus -Url "$Fb2Base/api/main-project/tools/execute" -Headers $fb2Headers -Method "POST" -Body @{
                    request_id = "main-smoke-permission-user-orders"
                    tool_name = "search_user_orders"
                    group_id = $GroupId
                    external_user_id = $ExternalUserId
                    arguments = @{
                        external_user_id = $ExternalUserId
                        limit = 1
                    }
                    reason = "main smoke permission boundary"
                }
                Assert-StatusCode $userToolWithoutHeader 403 "permission: user order tool requires current user header"

                Start-Sleep -Seconds 1
                $permissionParams = [System.Collections.Generic.List[string]]::new()
                Add-QueryParam -Params $permissionParams -Name "from" -Value $permissionSince
                Add-QueryParam -Params $permissionParams -Name "group_id" -Value $GroupId
                $permissionQuery = $permissionParams -join $amp
                $permissionUrl = "$Fb2Base/api/main-project/context/permission-summary"
                if ($permissionQuery) {
                    $permissionUrl = "${permissionUrl}?$permissionQuery"
                }
                $permissionSummary = Invoke-Json -Url $permissionUrl -Headers $fb2Headers
                Assert-True ($permissionSummary.success -eq $true) "permission summary"

                $permissionData = $permissionSummary.data
                $totalBlocks = Get-NestedNumber $permissionData @("total_blocks", "summary.total_blocks", "permission_summary.total_blocks")
                $missingUserBlocks = Get-NestedNumber $permissionData @("missing_external_user_id_count", "summary.missing_external_user_id_count", "permission_summary.missing_external_user_id_count")
                $platformScopeBlocks = Get-NestedNumber $permissionData @("platform_scope_count", "summary.platform_scope_count", "permission_summary.platform_scope_count")
                Assert-True ($null -ne $totalBlocks -and $totalBlocks -ge 4) "permission summary total blocks" "value=$totalBlocks"
                Assert-True ($null -ne $missingUserBlocks -and $missingUserBlocks -ge 3) "permission summary user blocks" "value=$missingUserBlocks"
                Assert-True ($null -ne $platformScopeBlocks -and $platformScopeBlocks -ge 1) "permission summary platform blocks" "value=$platformScopeBlocks"
            } catch {
                Fail "permission boundaries" $_.Exception.Message
            }
        }
    }

    if ($qualityCheckRequested) {
        Write-Output ""
        Write-Output "== fb2 quality gates =="

        try {
            $qualityParams = [System.Collections.Generic.List[string]]::new()
            Add-QueryParam -Params $qualityParams -Name "group_id" -Value $GroupId
            Add-QueryParam -Params $qualityParams -Name "from" -Value $QualitySince
            Add-QueryParam -Params $qualityParams -Name "to" -Value $QualityUntil
            $qualityQuery = $qualityParams -join $amp
            $qualityUrl = "$Fb2Base/api/main-project/context/quality-summary"
            if ($qualityQuery) {
                $qualityUrl = "${qualityUrl}?$qualityQuery"
            }

            $quality = Invoke-Json -Url $qualityUrl -Headers $fb2Headers
            $metrics = $quality.data.quality_metrics
            $feedbackSummary = $quality.data.feedback_summary
            $auditSummary = $quality.data.audit_summary

            Assert-True ($quality.success -eq $true) "quality summary"
            Assert-True ($null -ne $metrics) "quality metrics present"
            Assert-True ($null -ne $feedbackSummary) "quality feedback summary present"
            Assert-True ($null -ne $auditSummary) "quality audit summary present"

            if ($null -ne $metrics) {
                Assert-True ([double]$metrics.missing_context_rate -le $MaxMissingContextRate) "quality missing_context_rate" "value=$($metrics.missing_context_rate) max=$MaxMissingContextRate"
                Assert-True ([double]$metrics.wrong_context_rate -le $MaxWrongContextRate) "quality wrong_context_rate" "value=$($metrics.wrong_context_rate) max=$MaxWrongContextRate"
                Assert-True ([double]$metrics.citation_unmatched_rate -le $MaxCitationUnmatchedRate) "quality citation_unmatched_rate" "value=$($metrics.citation_unmatched_rate) max=$MaxCitationUnmatchedRate"
                Assert-True ([double]$metrics.large_context_pack_rate -le $MaxLargeContextPackRate) "quality large_context_pack_rate" "value=$($metrics.large_context_pack_rate) max=$MaxLargeContextPackRate"
            }

            if ($null -ne $feedbackSummary) {
                Assert-True ([int64]$feedbackSummary.total_feedback -ge $MinFeedbackCount) "quality feedback count" "value=$($feedbackSummary.total_feedback) min=$MinFeedbackCount"
                Assert-True ([int64]$feedbackSummary.matched_cited_source_count -ge $MinMatchedCitedSourceCount) "quality matched cited sources" "value=$($feedbackSummary.matched_cited_source_count) min=$MinMatchedCitedSourceCount"
                Assert-True ([int64]$feedbackSummary.unmatched_cited_source_count -eq 0) "quality unmatched cited sources" "value=$($feedbackSummary.unmatched_cited_source_count)"
                Assert-True ([int64]$feedbackSummary.missing_context_count -eq 0) "quality missing context count" "value=$($feedbackSummary.missing_context_count)"
                Assert-True ([int64]$feedbackSummary.wrong_context_count -eq 0) "quality wrong context count" "value=$($feedbackSummary.wrong_context_count)"
            }

            if ($RequireFeedbackCoverage -or $MinFeedbackCount -gt 0 -or $MinMatchedCitedSourceCount -gt 0) {
                $feedbackParams = [System.Collections.Generic.List[string]]::new()
                Add-QueryParam -Params $feedbackParams -Name "group_id" -Value $GroupId
                Add-QueryParam -Params $feedbackParams -Name "from" -Value $QualitySince
                Add-QueryParam -Params $feedbackParams -Name "to" -Value $QualityUntil
                Add-QueryParam -Params $feedbackParams -Name "limit" -Value ([string]$QualityFeedbackSampleLimit)
                $feedbackQuery = $feedbackParams -join $amp
                $feedbacks = Invoke-Json -Url "$Fb2Base/api/main-project/context/feedbacks?$feedbackQuery" -Headers $fb2Headers
                Assert-True ($feedbacks.success -eq $true) "quality feedback samples"
                Assert-True ([int]$feedbacks.data.count -gt 0) "quality feedback sample count" "count=$($feedbacks.data.count)"
            }

            if ($RequireNonSyntheticQualityReadiness) {
                $nonSyntheticParams = [System.Collections.Generic.List[string]]::new()
                Add-QueryParam -Params $nonSyntheticParams -Name "group_id" -Value $GroupId
                Add-QueryParam -Params $nonSyntheticParams -Name "external_user_id" -Value $ExternalUserId
                Add-QueryParam -Params $nonSyntheticParams -Name "exclude_synthetic" -Value "true"
                Add-QueryParam -Params $nonSyntheticParams -Name "from" -Value $QualitySince
                Add-QueryParam -Params $nonSyntheticParams -Name "to" -Value $QualityUntil
                $nonSyntheticQuery = $nonSyntheticParams -join $amp

                $nonSyntheticQuality = Invoke-Json -Url "$Fb2Base/api/main-project/context/quality-summary?$nonSyntheticQuery" -Headers $fb2Headers
                $nonSyntheticFeedback = Invoke-Json -Url "$Fb2Base/api/main-project/context/feedback-summary?$nonSyntheticQuery" -Headers $fb2Headers
                $nonSyntheticAdoption = Invoke-Json -Url "$Fb2Base/api/main-project/context/opinion-adoption-summary?$nonSyntheticQuery" -Headers $fb2Headers

                Assert-True ($nonSyntheticQuality.success -eq $true) "quality non-synthetic summary"
                Assert-True ($nonSyntheticFeedback.success -eq $true) "quality non-synthetic feedback summary"
                Assert-True ($nonSyntheticAdoption.success -eq $true) "quality non-synthetic adoption summary"

                $nonSyntheticFeedbackCount = Get-NestedNumber $nonSyntheticFeedback.data @("summary.total_feedback", "feedback_summary.total_feedback", "total_feedback")
                $nonSyntheticQualityFeedbackCount = Get-NestedNumber $nonSyntheticQuality.data @("feedback_summary.total_feedback", "summary.total_feedback", "total_feedback")
                $nonSyntheticAdoptionCount = Get-NestedNumber $nonSyntheticAdoption.data @("summary.total_adoptions", "opinion_adoption_summary.total_adoptions", "total_adoptions")
                $nonSyntheticMemoryRefs = Get-NestedNumber $nonSyntheticAdoption.data @("summary.total_memory_refs", "opinion_adoption_summary.total_memory_refs", "total_memory_refs")

                Assert-True ($null -ne $nonSyntheticFeedbackCount) "quality non-synthetic feedback metric" "value=$nonSyntheticFeedbackCount"
                Assert-True ($null -ne $nonSyntheticQualityFeedbackCount) "quality non-synthetic feedback alignment metric" "value=$nonSyntheticQualityFeedbackCount"
                Assert-True ($null -ne $nonSyntheticAdoptionCount) "quality non-synthetic adoption metric" "value=$nonSyntheticAdoptionCount"
                Assert-True ($nonSyntheticQualityFeedbackCount -eq $nonSyntheticFeedbackCount) "quality non-synthetic summary alignment" "quality=$nonSyntheticQualityFeedbackCount feedback=$nonSyntheticFeedbackCount"
                Assert-True ($nonSyntheticFeedbackCount -ge $MinNonSyntheticFeedbackCount) "quality non-synthetic feedback count" "value=$nonSyntheticFeedbackCount min=$MinNonSyntheticFeedbackCount"
                Assert-True ($nonSyntheticAdoptionCount -ge $MinOpinionAdoptionCount) "quality non-synthetic adoption count" "value=$nonSyntheticAdoptionCount min=$MinOpinionAdoptionCount"
                Assert-True ($null -ne $nonSyntheticMemoryRefs) "quality non-synthetic memory refs" "value=$nonSyntheticMemoryRefs"
            }
        } catch {
            Fail "fb2 quality gates" $_.Exception.Message
        }
    }
}

Write-Output ""
Write-Output "== Summary =="
Write-Output "failed=$script:Failed skipped=$script:Skipped"
if ($RequireNoSkips -and $script:Skipped -gt 0) {
    Write-Check "FAIL" "no skipped checks" "skipped=$script:Skipped"
    exit 1
}
if ($script:Failed -gt 0) {
    exit 1
}

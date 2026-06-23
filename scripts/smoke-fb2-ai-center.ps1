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
    [switch]$AllowHistoricalQualityDebt,
    [switch]$CheckDomainProjection,
    [switch]$SkipVoiceContractChecks,
    [switch]$RequireNoSkips,
    [string]$SummaryPath = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-ai-center-contract-smoke-summary.ps1")
. (Join-Path $PSScriptRoot "direct-network.ps1")

Set-ElonProjectDirectNetwork

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
$script:Fb2ContractSmokeChecks = [System.Collections.Generic.List[object]]::new()

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
    [void]$script:Fb2ContractSmokeChecks.Add([ordered]@{
        status = $Status
        name = $Name
        detail = $Detail
    })
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

function Get-Fb2ToolIdVariants {
    param([object]$Value)

    if ($null -eq $Value) {
        return @()
    }

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return @()
    }

    $variants = @($text)
    if ($text.StartsWith("fb2.", [System.StringComparison]::OrdinalIgnoreCase)) {
        $variants += $text.Substring(4)
    }
    @($variants)
}

function Get-Fb2ManifestToolIds {
    param([object]$Manifest)

    $ids = @()
    foreach ($id in @($Manifest.data.tool_ids)) {
        $ids += Get-Fb2ToolIdVariants $id
    }
    foreach ($id in @($Manifest.data.tool_contract.tool_ids)) {
        $ids += Get-Fb2ToolIdVariants $id
    }
    foreach ($endpoint in @($Manifest.data.tool_contract.endpoints)) {
        if ($endpoint -is [string]) {
            $ids += Get-Fb2ToolIdVariants $endpoint
            continue
        }
        foreach ($field in @("id", "tool_id", "name", "key", "execute_tool_name")) {
            $property = $endpoint.PSObject.Properties[$field]
            if ($null -ne $property -and -not [string]::IsNullOrWhiteSpace([string]$property.Value)) {
                $ids += Get-Fb2ToolIdVariants $property.Value
            }
        }
    }
    @($ids | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
}

function Get-Fb2ReadinessStatus {
    param([object]$Readiness)

    $candidates = @(
        $Readiness.data.status,
        $Readiness.data.readiness.status,
        $Readiness.data.readiness_status,
        $Readiness.data.context_status,
        $Readiness.data.context_readiness.status
    )

    @($candidates | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -First 1)
}

function Test-Fb2ReadinessAllowedForMode {
    param(
        [string]$Status,
        [bool]$StrictFinalAcceptance,
        [bool]$StrictDataOnlyAcceptance
    )

    if ([string]::IsNullOrWhiteSpace($Status)) {
        return $false
    }

    if ($StrictFinalAcceptance) {
        return $Status -eq "ready"
    }

    if ($StrictDataOnlyAcceptance) {
        return @("ready", "partial") -contains $Status
    }

    return @("ready", "partial", "degraded", "blocked", "unavailable") -contains $Status
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
        $params = Add-ElonProjectDirectRequestParameters -Params $params -CommandName "Invoke-RestMethod"
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
    $params = Add-ElonProjectDirectRequestParameters -Params $params -CommandName "Invoke-WebRequest"
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

function Assert-AiCenterSelfTestCondition {
    param(
        [string]$Name,
        [bool]$Condition,
        [string]$Details = ""
    )

    if ($Condition) {
        Write-Output "OK`tself-test $Name`t$Details"
    } else {
        $script:SelfTestFailed += 1
        Write-Output "FAIL`tself-test $Name`t$Details"
    }
}

function Test-Fb2HistoricalQualityDebtAllowed {
    param(
        [bool]$Allowed,
        [string]$Since,
        [string]$Until
    )

    # 历史质量债务只能在没有时间窗的累计巡检里降级为观察项；带时间窗的当前批次仍然严格要求 0。
    $Allowed -and [string]::IsNullOrWhiteSpace($Since) -and [string]::IsNullOrWhiteSpace($Until)
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

        $readinessShape = [pscustomobject]@{
            data = [pscustomobject]@{
                readiness = [pscustomobject]@{
                    status = "partial"
                }
            }
        }
        $readinessStatus = Get-Fb2ReadinessStatus $readinessShape
        Assert-AiCenterSelfTestCondition "reads nested fb2 readiness status" ($readinessStatus -eq "partial") "status=$readinessStatus"
        Assert-AiCenterSelfTestCondition "final acceptance requires ready readiness" (Test-Fb2ReadinessAllowedForMode "ready" $true $false) "status=ready"
        Assert-AiCenterSelfTestCondition "final acceptance rejects partial readiness" (-not (Test-Fb2ReadinessAllowedForMode "partial" $true $false)) "status=partial"
        Assert-AiCenterSelfTestCondition "data-only acceptance allows partial readiness" (Test-Fb2ReadinessAllowedForMode "partial" $false $true) "status=partial"
        Assert-AiCenterSelfTestCondition "data-only acceptance rejects degraded readiness" (-not (Test-Fb2ReadinessAllowedForMode "degraded" $false $true)) "status=degraded"

        $manifestShape = [pscustomobject]@{
            data = [pscustomobject]@{
                tool_ids = @("fb2.context_pack")
                tool_contract = [pscustomobject]@{
                    tool_ids = @("fb2.today_matches")
                    endpoints = @(
                        [pscustomobject]@{ id = "fb2.group_opinion_summary"; execute_tool_name = "fb2.group_opinion_summary" },
                        [pscustomobject]@{ id = "fb2.context_quality_summary"; path = "/api/main-project/context/quality-summary" },
                        "fb2.tool_manifest"
                    )
                }
            }
        }
        $manifestIds = Get-Fb2ManifestToolIds $manifestShape
        Assert-AiCenterSelfTestCondition "normalizes fb2-prefixed manifest IDs" (($manifestIds -contains "context_pack") -and ($manifestIds -contains "today_matches") -and ($manifestIds -contains "group_opinion_summary") -and ($manifestIds -contains "tool_manifest")) "ids=$($manifestIds -join ',')"
        Assert-AiCenterSelfTestCondition "allows historical quality debt without window" (Test-Fb2HistoricalQualityDebtAllowed $true "" "") "mode=cumulative"
        Assert-AiCenterSelfTestCondition "rejects historical quality debt with since window" (-not (Test-Fb2HistoricalQualityDebtAllowed $true "2026-06-23T00:00:00Z" "")) "mode=windowed"
        Assert-AiCenterSelfTestCondition "rejects historical quality debt unless explicit" (-not (Test-Fb2HistoricalQualityDebtAllowed $false "" "")) "mode=strict"

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
    $healthParams = @{
        Uri = "$MainBase/health"
        UseBasicParsing = $true
        TimeoutSec = 10
    }
    $healthParams = Add-ElonProjectDirectRequestParameters -Params $healthParams -CommandName "Invoke-WebRequest"
    $health = (Invoke-WebRequest @healthParams).Content.Trim()
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
    "context_audit_summary",
    "tool_manifest"
)
$liveToolIds = @()

try {
    $contract = Invoke-Json "$MainBase/api/external/apps/fb2/context-contract"
    $policy = $contract.live_tool_manifest.main_project_tool_execution_policy
    $liveToolIds = @($contract.live_tool_manifest.tool_ids)
    $answerPolicy = $contract.answer_policy_contract
    $contextPackTemplate = $contract.context_pack_template_contract
    $projectionContract = $contract.domain_context_projection_contract
    $projectionLayerContract = $contract.context_projection_layer_contract
    $domainDataBlueprint = $contract.domain_data_blueprint_contract
    $domainContextIndex = $contract.domain_context_index_contract
    $groupChatEvidenceContract = $contract.group_chat_evidence_contract
    $toolExecutionContract = $contract.tool_execution_contract
    $toolExecutionPlan = $toolExecutionContract.main_project_execution_result.plan
    $projectionSections = @($projectionContract.required_sections)
    $projectionSectionIds = @($projectionSections | ForEach-Object { $_.id })
    $projectionSourceKinds = @($projectionContract.source_registry.required_kinds)
    $projectionQualityHistoryKinds = @($projectionContract.source_registry.quality_history_kinds | ForEach-Object { $_.kind })
    $projectionAntiPatterns = @($projectionContract.anti_patterns)
    $projectionRetrievalFields = @($projectionContract.retrieval_projection.recommended_fields)
    $projectionRetrievalItemFields = @($projectionContract.retrieval_projection.item_shape.required_fields)
    $projectionPermissions = @($projectionContract.permission_projection)
    $projectionQualityRoutes = @($projectionContract.quality_closure.required_feedback_routes)
    $projectionReadiness = $projectionContract.quality_closure.minimum_non_synthetic_ready
    $projectionLayerLaneIds = @($projectionLayerContract.domain_lanes | ForEach-Object { $_.id })
    $projectionLayerIndexIds = @($projectionLayerContract.domain_indexes | ForEach-Object { $_.id })
    $projectionLayerScenarioIds = @($projectionLayerContract.user_scenarios | ForEach-Object { $_.id })
    $projectionLayerForbidden = @($projectionLayerContract.forbidden_outputs)
    $projectionLayerNotAllowed = @($projectionLayerContract.ai_facing_payload.not_allowed)
    $projectionLayerGroupFields = @($projectionLayerContract.group_chat_evidence.required_fields)
    $projectionLayerRetrievalFields = @($projectionLayerContract.retrieval_evidence_contract.required_fields)
    $domainDataBlueprintLaneIds = @($domainDataBlueprint.lanes | ForEach-Object { $_.id })
    $domainDataBlueprintSections = @($domainDataBlueprint.required_context_pack_sections)
    $domainDataBlueprintMetadata = @($domainDataBlueprint.required_metadata)
    $domainDataBlueprintAntiPatterns = @($domainDataBlueprint.anti_patterns)
    $domainContextIndexIds = @($domainContextIndex.indexes | ForEach-Object { $_.id })
    $domainContextIndexInputs = @($domainContextIndex.required_query_inputs)
    $domainContextIndexMetrics = @($domainContextIndex.required_metrics)
    $domainContextIndexNotAllowed = @($domainContextIndex.index_output_boundary.not_allowed)
    $domainContextIndexRetrievalFields = @($domainContextIndex.retrieval_evidence_output_shape.required_fields)
    $groupChatEvidenceFields = @($groupChatEvidenceContract.required_group_message_fields)
    $groupChatVisibleEvidence = @($groupChatEvidenceContract.required_visible_flow_evidence)
    $toolResultEnvelopeContract = $contract.tool_result_envelope_contract
    $toolResultRequiredFields = @($toolResultEnvelopeContract.normalized_envelope.required_fields)
    $toolResultBusinessSourceKinds = @($toolResultEnvelopeContract.source_registry.business_source_kinds)
    $toolResultQualityHistoryKinds = @($toolResultEnvelopeContract.source_registry.quality_history_kinds | ForEach-Object { $_.kind })
    $toolResultGroundingStatuses = @($toolResultEnvelopeContract.grounding.statuses | ForEach-Object { $_.status })
    $toolResultGroundingFields = @($toolResultEnvelopeContract.grounding.required_fields)
    $evalScenarios = @($answerPolicy.eval_scenarios)
    $evalScenarioIds = @($evalScenarios | ForEach-Object { $_.id })
    $templateSections = @($contextPackTemplate.required_section_order)
    $templateMetadata = @($contextPackTemplate.required_metadata)
    $templateBusinessKinds = @($contextPackTemplate.citation_source_shape.business_source_kinds)
    $templateQualityKinds = @($contextPackTemplate.citation_source_shape.quality_history_kinds | ForEach-Object { $_.kind })
    $templateRetrievalFields = @($contextPackTemplate.retrieval_evidence_item_shape.required_fields)
    Assert-True ($contract.live_tool_manifest.status -eq "ready") "live manifest ready" "tool_count=$($contract.live_tool_manifest.tool_count)"
    Assert-True ($policy.schema -eq "external_app.live_tool_execution_policy.v1") "live manifest execution policy"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "search_matches") -and ($policy.chat_auto_executable_tool_ids -contains "search_group_opinions")) "auto executable core tools"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "match_analysis_brief") -and ($policy.chat_auto_executable_tool_ids -contains "group_opinion_summary")) "auto executable aggregate tools"
    Assert-True ($policy.manifest_only_tool_ids -contains "record_context_feedback") "callback tool is not chat-auto-executable"
    Assert-True (@($policy.main_project_allowed_missing_tool_ids).Count -eq 0) "no allowed tool missing in live fb2 manifest"
    foreach ($toolId in $requiredFb2ToolIds) {
        Assert-ContainsValue $liveToolIds $toolId "live manifest required tool: $toolId"
    }
    Assert-True ($toolExecutionContract.schema -eq "fb2.tool_execution.v1") "tool execution contract schema" "$($toolExecutionContract.schema)"
    Assert-True ($toolExecutionContract.main_project_execution_result.schema -eq "external_app.executed_tools.v1") "tool execution result schema" "$($toolExecutionContract.main_project_execution_result.schema)"
    Assert-True ($toolExecutionPlan.schema -eq "external_app.tool_plan.v1") "tool execution plan schema" "$($toolExecutionPlan.schema)"
    Assert-True ($toolExecutionPlan.strategy -eq "deterministic_fb2_chat_v1") "tool execution plan strategy" "$($toolExecutionPlan.strategy)"
    $domainScenarioSelectionContract = [string]$toolExecutionPlan.domain_scenario_selection
    Assert-True ($domainScenarioSelectionContract.Contains("fb2.domain_scenario_selection.v1")) "tool execution plan domain scenario selection" $domainScenarioSelectionContract
    foreach ($fieldName in @("primary_tools", "required_citations", "permission_scope", "forbidden_outputs")) {
        Assert-True ($domainScenarioSelectionContract.Contains($fieldName)) "tool execution plan domain scenario selection field: $fieldName" $domainScenarioSelectionContract
    }
    Assert-True ($answerPolicy.schema -eq "fb2.answer_policy.v1") "answer policy schema"
    Assert-True (@($answerPolicy.canonical_eval_questions).Count -ge 6) "answer policy canonical eval questions" "count=$(@($answerPolicy.canonical_eval_questions).Count)"
    Assert-True (($evalScenarioIds -contains "today_matches_analysis") -and ($evalScenarioIds -contains "my_ticket_analysis")) "answer policy core eval scenarios"
    Assert-True (($evalScenarioIds -contains "platform_order_risk") -and ($evalScenarioIds -contains "group_opinion_summary")) "answer policy aggregate eval scenarios"
    Assert-True (($evalScenarioIds -contains "selected_message_review") -and ($evalScenarioIds -contains "source_reference_audit")) "answer policy audit eval scenarios"

    Assert-True ($contextPackTemplate.schema -eq "fb2.context_pack_template.v1") "context pack template schema" "$($contextPackTemplate.schema)"
    Assert-True ($contextPackTemplate.complete -eq $true) "context pack template complete" "complete=$($contextPackTemplate.complete)"
    Assert-True ($contextPackTemplate.body.wrapper -eq "fb2_context_pack") "context pack template wrapper" "$($contextPackTemplate.body.wrapper)"
    Assert-True ($contextPackTemplate.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") "context pack template first phase" "$($contextPackTemplate.first_phase_delivery)"
    Assert-True ($contextPackTemplate.mcp_status -eq "future_wrapper_not_first_phase_fact_source") "context pack template mcp status" "$($contextPackTemplate.mcp_status)"
    foreach ($sectionId in @("usage_boundary", "match_facts", "user_order_slice", "platform_order_summary", "group_opinion_slice", "retrieval_evidence", "quality_feedback")) {
        Assert-ContainsValue $templateSections $sectionId "context pack template section: $sectionId"
    }
    foreach ($metadataName in @("context_pack_version", "generated_at", "context_audit_id", "citation_sources", "metrics", "preflight_readiness")) {
        Assert-ContainsValue $templateMetadata $metadataName "context pack template metadata: $metadataName"
    }
    foreach ($sourceKind in @("match", "odds", "user_order", "ticket", "group_message", "opinion_memory", "platform_order_summary")) {
        Assert-ContainsValue $templateBusinessKinds $sourceKind "context pack template source kind: $sourceKind"
    }
    Assert-True (-not ($templateBusinessKinds -contains "feedback")) "context pack template business sources exclude feedback"
    Assert-ContainsValue $templateQualityKinds "feedback" "context pack template quality history kind: feedback"
    Assert-ContainsValue $templateQualityKinds "opinion_adoption" "context pack template quality history kind: opinion adoption"
    Assert-ContainsValue @($contextPackTemplate.body.not_allowed) "full_database_dump" "context pack template anti-pattern: full database dump"
    Assert-ContainsValue @($contextPackTemplate.answer_boundaries) "不得承诺投注命中，不得建议重注或梭哈。" "context pack template betting boundary"
    Assert-True ($contextPackTemplate.retrieval_evidence_item_shape.schema -eq "fb2.retrieval_evidence_item.v1") "context pack template retrieval evidence shape" "$($contextPackTemplate.retrieval_evidence_item_shape.schema)"
    foreach ($fieldName in @("source_id", "source_kind", "lane_id", "index_id", "reason", "freshness", "permission_scope", "citation_source_id")) {
        Assert-ContainsValue $templateRetrievalFields $fieldName "context pack template retrieval evidence field: $fieldName"
    }

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
    Assert-True (-not ($projectionSourceKinds -contains "feedback")) "domain projection business sources exclude feedback"
    Assert-True (-not ($projectionSourceKinds -contains "opinion_adoption")) "domain projection business sources exclude opinion adoption"
    Assert-ContainsValue $projectionQualityHistoryKinds "feedback" "domain projection quality history kind: feedback"
    Assert-ContainsValue $projectionQualityHistoryKinds "opinion_adoption" "domain projection quality history kind: opinion adoption"
    Assert-ContainsValue $projectionAntiPatterns "raw_embedding_dump" "domain projection anti-pattern: raw embedding dump"
    Assert-ContainsValue $projectionAntiPatterns "platform_order_detail_leak" "domain projection anti-pattern: platform order leak"
    Assert-ContainsValue $projectionRetrievalFields "topic_hint" "domain projection retrieval field: topic hint"
    Assert-ContainsValue $projectionRetrievalFields "match_reason" "domain projection retrieval field: match reason"
    Assert-ContainsValue $projectionRetrievalFields "permission_scope" "domain projection retrieval field: permission scope"
    Assert-ContainsValue $projectionRetrievalFields "truncated" "domain projection retrieval field: truncated"
    Assert-True ($projectionContract.retrieval_projection.item_shape.schema -eq "fb2.retrieval_evidence_item.v1") "domain projection retrieval evidence shape" "$($projectionContract.retrieval_projection.item_shape.schema)"
    foreach ($fieldName in @("source_id", "source_kind", "lane_id", "index_id", "reason", "freshness", "permission_scope", "citation_source_id")) {
        Assert-ContainsValue $projectionRetrievalItemFields $fieldName "domain projection retrieval evidence field: $fieldName"
    }
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/feedback" "domain projection quality route: feedback"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/feedback-summary" "domain projection quality route: feedback summary"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/opinion-adoption-summary" "domain projection quality route: opinion adoption summary"
    Assert-ContainsValue $projectionQualityRoutes "/api/main-project/context/quality-summary" "domain projection quality route: quality summary"
    Assert-True ([int]$projectionReadiness.feedback_count -ge 1) "domain projection readiness: feedback count" "feedback_count=$($projectionReadiness.feedback_count)"
    Assert-True ([int]$projectionReadiness.opinion_adoption_count -ge 1) "domain projection readiness: opinion adoption count" "opinion_adoption_count=$($projectionReadiness.opinion_adoption_count)"
    Assert-True ([string]$projectionReadiness.opinion_memory_ref_count -eq "present") "domain projection readiness: opinion memory refs" "opinion_memory_ref_count=$($projectionReadiness.opinion_memory_ref_count)"
    Assert-Fb2DomainScenarioMatrixContract -ScenarioMatrix $projectionContract.domain_scenario_matrix

    Assert-True ($projectionLayerContract.schema -eq "fb2.main_project.context_projection_layer.v1") "context projection layer schema" "$($projectionLayerContract.schema)"
    Assert-True ($projectionLayerContract.complete -eq $true) "context projection layer complete" "complete=$($projectionLayerContract.complete)"
    Assert-True ($projectionLayerContract.ai_facing_payload.wrapper -eq "fb2_context_pack") "context projection layer wrapper" "$($projectionLayerContract.ai_facing_payload.wrapper)"
    Assert-True ($projectionLayerContract.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") "context projection layer first phase" "$($projectionLayerContract.first_phase_delivery)"
    Assert-True ($projectionLayerContract.mcp_status -eq "future_wrapper_not_first_phase_fact_source") "context projection layer mcp status" "$($projectionLayerContract.mcp_status)"
    Assert-True ($projectionLayerContract.stores_fb2_business_data_in_main_project -eq $false) "context projection layer no main-project data copy" "stores=$($projectionLayerContract.stores_fb2_business_data_in_main_project)"
    Assert-True ([int]$projectionLayerContract.domain_lane_count -eq 6) "context projection layer lane count" "lane_count=$($projectionLayerContract.domain_lane_count)"
    foreach ($laneId in @("match_facts_and_odds", "current_user_tickets", "platform_order_summary", "group_opinions", "opinion_learning_loop", "quality_feedback_audit")) {
        Assert-ContainsValue $projectionLayerLaneIds $laneId "context projection layer lane: $laneId"
    }
    Assert-True ([int]$projectionLayerContract.domain_index_count -eq 8) "context projection layer index count" "index_count=$($projectionLayerContract.domain_index_count)"
    foreach ($indexId in @("match_index", "odds_snapshot_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "opinion_memory_index", "context_audit_index", "feedback_quality_index")) {
        Assert-ContainsValue $projectionLayerIndexIds $indexId "context projection layer index: $indexId"
    }
    Assert-True ([int]$projectionLayerContract.user_scenario_count -eq 7) "context projection layer scenario count" "scenario_count=$($projectionLayerContract.user_scenario_count)"
    foreach ($scenarioId in @("today_matches_analysis", "my_ticket_analysis", "platform_order_risk", "group_opinion_summary", "selected_message_review", "group_discussion_summary_post", "source_reference_audit")) {
        Assert-ContainsValue $projectionLayerScenarioIds $scenarioId "context projection layer scenario: $scenarioId"
    }
    Assert-ContainsValue $projectionLayerForbidden "fabricated_odds" "context projection layer forbidden: fabricated odds"
    Assert-ContainsValue $projectionLayerForbidden "raw_embedding_dump" "context projection layer forbidden: raw embedding dump"
    Assert-ContainsValue $projectionLayerNotAllowed "full_database_dump" "context projection layer not allowed: full database dump"
    Assert-True ($projectionLayerContract.retrieval_evidence_contract.schema -eq "fb2.retrieval_evidence_item.v1") "context projection layer retrieval evidence shape" "$($projectionLayerContract.retrieval_evidence_contract.schema)"
    Assert-ContainsValue $projectionLayerRetrievalFields "citation_source_id" "context projection layer retrieval evidence field: citation source id"
    Assert-True ($projectionLayerContract.group_chat_evidence.method -eq "direct_api_read") "context projection layer group direct read" "$($projectionLayerContract.group_chat_evidence.method)"
    Assert-True ($projectionLayerContract.group_chat_evidence.screenshots_accepted -eq $false) "context projection layer rejects screenshots" "screenshots_accepted=$($projectionLayerContract.group_chat_evidence.screenshots_accepted)"
    Assert-ContainsValue $projectionLayerGroupFields "text_sha256" "context projection layer group field: text sha256"

    Assert-True ($domainDataBlueprint.schema -eq "fb2.main_project.domain_data_blueprint.v1") "domain data blueprint schema" "$($domainDataBlueprint.schema)"
    Assert-True ($domainDataBlueprint.complete -eq $true) "domain data blueprint complete" "complete=$($domainDataBlueprint.complete)"
    Assert-True ($domainDataBlueprint.context_format -eq "xml_wrapped_markdown_context_pack_with_json_metadata") "domain data blueprint context format" "$($domainDataBlueprint.context_format)"
    Assert-True ($domainDataBlueprint.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") "domain data blueprint first phase" "$($domainDataBlueprint.first_phase_delivery)"
    Assert-True ($domainDataBlueprint.mcp_status -eq "future_wrapper_not_first_phase_fact_source") "domain data blueprint mcp status" "$($domainDataBlueprint.mcp_status)"
    Assert-True ($domainDataBlueprint.stores_fb2_business_data_in_main_project -eq $false) "domain data blueprint no main-project data copy" "stores=$($domainDataBlueprint.stores_fb2_business_data_in_main_project)"
    Assert-True ([int]$domainDataBlueprint.lane_count -eq 6) "domain data blueprint lane count" "lane_count=$($domainDataBlueprint.lane_count)"
    Assert-ContainsValue $domainDataBlueprintLaneIds "match_facts_and_odds" "domain data blueprint lane: match facts and odds"
    Assert-ContainsValue $domainDataBlueprintLaneIds "current_user_tickets" "domain data blueprint lane: current user tickets"
    Assert-ContainsValue $domainDataBlueprintLaneIds "platform_order_summary" "domain data blueprint lane: platform summary"
    Assert-ContainsValue $domainDataBlueprintLaneIds "group_opinions" "domain data blueprint lane: group opinions"
    Assert-ContainsValue $domainDataBlueprintLaneIds "opinion_learning_loop" "domain data blueprint lane: opinion learning"
    Assert-ContainsValue $domainDataBlueprintLaneIds "quality_feedback_audit" "domain data blueprint lane: quality audit"
    Assert-ContainsValue $domainDataBlueprintSections "group_opinion_slice" "domain data blueprint section: group opinion slice"
    Assert-ContainsValue $domainDataBlueprintMetadata "citation_sources" "domain data blueprint metadata: citation sources"
    Assert-ContainsValue $domainDataBlueprintAntiPatterns "full_database_dump" "domain data blueprint anti-pattern: full database dump"

    Assert-True ($domainContextIndex.schema -eq "fb2.main_project.domain_context_index.v1") "domain context index schema" "$($domainContextIndex.schema)"
    Assert-True ($domainContextIndex.complete -eq $true) "domain context index complete" "complete=$($domainContextIndex.complete)"
    Assert-True ($domainContextIndex.stores_fb2_business_data_in_main_project -eq $false) "domain context index no main-project data copy" "stores=$($domainContextIndex.stores_fb2_business_data_in_main_project)"
    Assert-True ([int]$domainContextIndex.index_count -eq 8) "domain context index count" "index_count=$($domainContextIndex.index_count)"
    foreach ($indexId in @("match_index", "odds_snapshot_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "opinion_memory_index", "context_audit_index", "feedback_quality_index")) {
        Assert-ContainsValue $domainContextIndexIds $indexId "domain context index: $indexId"
    }
    Assert-ContainsValue $domainContextIndexInputs "topic_hint" "domain context index input: topic hint"
    Assert-ContainsValue $domainContextIndexInputs "external_user_id_when_user_orders_are_requested" "domain context index input: external user id"
    Assert-ContainsValue $domainContextIndexMetrics "budget_status" "domain context index metric: budget status"
    Assert-ContainsValue $domainContextIndexNotAllowed "raw_embedding_dump" "domain context index anti-pattern: raw embedding dump"
    Assert-ContainsValue $domainContextIndexNotAllowed "full_database_dump" "domain context index anti-pattern: full database dump"
    Assert-True ($domainContextIndex.retrieval_evidence_output_shape.schema -eq "fb2.retrieval_evidence_item.v1") "domain context index retrieval evidence shape" "$($domainContextIndex.retrieval_evidence_output_shape.schema)"
    Assert-ContainsValue $domainContextIndexRetrievalFields "index_id" "domain context index retrieval evidence field: index id"
    Assert-ContainsValue $domainContextIndexRetrievalFields "citation_source_id" "domain context index retrieval evidence field: citation source id"

    Assert-True ($groupChatEvidenceContract.schema -eq "fb2.main_project.group_chat_evidence.v1") "group chat evidence schema" "$($groupChatEvidenceContract.schema)"
    Assert-True ($groupChatEvidenceContract.group_chat_test_method -eq "direct_api_read") "group chat evidence direct read" "$($groupChatEvidenceContract.group_chat_test_method)"
    Assert-True ($groupChatEvidenceContract.screenshots_accepted -eq $false) "group chat evidence rejects screenshots" "screenshots_accepted=$($groupChatEvidenceContract.screenshots_accepted)"
    Assert-True ($groupChatEvidenceContract.write_policy.no_write_preflight -eq $true) "group chat evidence no-write preflight" "no_write_preflight=$($groupChatEvidenceContract.write_policy.no_write_preflight)"
    Assert-True ($groupChatEvidenceContract.write_policy.visible_message_test_requires_authorization -eq $true) "group chat evidence visible write authorization" "visible_message_test_requires_authorization=$($groupChatEvidenceContract.write_policy.visible_message_test_requires_authorization)"
    Assert-ContainsValue $groupChatEvidenceFields "message_id" "group chat evidence field: message id"
    Assert-ContainsValue $groupChatEvidenceFields "text_len" "group chat evidence field: text length"
    Assert-ContainsValue $groupChatEvidenceFields "text_sha256" "group chat evidence field: text sha256"
    Assert-ContainsValue $groupChatVisibleEvidence "visible_mention_ai_reply_read" "group chat evidence: mention reply read"
    Assert-ContainsValue $groupChatVisibleEvidence "selected_message_ai_reply_read" "group chat evidence: selected reply read"
    Assert-ContainsValue $groupChatVisibleEvidence "summary_post_read" "group chat evidence: summary post read"
    Assert-ContainsValue $groupChatVisibleEvidence "feedback_quality_read" "group chat evidence: feedback quality read"

    Assert-True ($toolResultEnvelopeContract.schema -eq "fb2.tool_result_envelope.v1") "tool result envelope schema" "$($toolResultEnvelopeContract.schema)"
    Assert-True ($toolResultEnvelopeContract.normalized_result_schema -eq "external_app.normalized_tool_result.v1") "tool result normalized schema" "$($toolResultEnvelopeContract.normalized_result_schema)"
    Assert-ContainsValue $toolResultRequiredFields "source_ids" "tool result required field: source ids"
    Assert-ContainsValue $toolResultRequiredFields "visibility" "tool result required field: visibility"
    Assert-ContainsValue $toolResultRequiredFields "grounding" "tool result required field: grounding"
    Assert-ContainsValue $toolResultGroundingFields "facts_allowed" "tool result grounding field: facts allowed"
    Assert-ContainsValue $toolResultGroundingStatuses "grounded" "tool result grounding status: grounded"
    Assert-ContainsValue $toolResultGroundingStatuses "weak" "tool result grounding status: weak"
    Assert-ContainsValue $toolResultGroundingStatuses "unsafe" "tool result grounding status: unsafe"
    Assert-ContainsValue $toolResultGroundingStatuses "unavailable" "tool result grounding status: unavailable"
    Assert-ContainsValue $toolResultBusinessSourceKinds "match" "tool result business source kind: match"
    Assert-ContainsValue $toolResultBusinessSourceKinds "user_order" "tool result business source kind: user order"
    Assert-ContainsValue $toolResultBusinessSourceKinds "platform_order_summary" "tool result business source kind: platform summary"
    Assert-True (-not ($toolResultBusinessSourceKinds -contains "feedback")) "tool result business sources exclude feedback"
    Assert-ContainsValue $toolResultQualityHistoryKinds "feedback" "tool result quality history kind: feedback"
    Assert-ContainsValue $toolResultQualityHistoryKinds "opinion_adoption" "tool result quality history kind: opinion adoption"

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
        $readinessValue = Get-Fb2ReadinessStatus $readiness
        Assert-True ([bool]$readinessValue) "fb2 authenticated readiness status" "status=$readinessValue"
        Assert-ContainsValue @("ready", "partial", "degraded", "blocked", "unavailable") $readinessValue "fb2 authenticated readiness status value"
        if ($FinalAcceptance -or $DataOnlyAcceptance) {
            $readinessAllowed = Test-Fb2ReadinessAllowedForMode $readinessValue ([bool]$FinalAcceptance) ([bool]$DataOnlyAcceptance)
            $readinessMode = if ($FinalAcceptance) { "final_acceptance" } else { "data_only_acceptance" }
            $readinessReason = if ($FinalAcceptance) { "requires_ready" } else { "allows_ready_or_partial" }
            Assert-True $readinessAllowed "fb2 authenticated readiness acceptable for acceptance" "mode=$readinessMode status=$readinessValue allowed=$readinessAllowed reason=$readinessReason"
        }

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

        $headParams = @{
            UseBasicParsing = $true
            Uri = $apkUrl
            Method = "Head"
            TimeoutSec = $RequestTimeoutSec
        }
        $headParams = Add-ElonProjectDirectRequestParameters -Params $headParams -CommandName "Invoke-WebRequest"
        $head = Invoke-WebRequest @headParams
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

            $historicalQualityDebtAllowed = Test-Fb2HistoricalQualityDebtAllowed $AllowHistoricalQualityDebt $QualitySince $QualityUntil
            if ($null -ne $metrics) {
                Assert-True ([double]$metrics.missing_context_rate -le $MaxMissingContextRate) "quality missing_context_rate" "value=$($metrics.missing_context_rate) max=$MaxMissingContextRate"
                if ($historicalQualityDebtAllowed) {
                    Pass "quality wrong_context_rate" "value=$($metrics.wrong_context_rate) max=$MaxWrongContextRate observation=historical_debt_allowed"
                } else {
                    Assert-True ([double]$metrics.wrong_context_rate -le $MaxWrongContextRate) "quality wrong_context_rate" "value=$($metrics.wrong_context_rate) max=$MaxWrongContextRate"
                }
                if ($historicalQualityDebtAllowed) {
                    Pass "quality citation_unmatched_rate" "value=$($metrics.citation_unmatched_rate) max=$MaxCitationUnmatchedRate observation=historical_debt_allowed"
                } else {
                    Assert-True ([double]$metrics.citation_unmatched_rate -le $MaxCitationUnmatchedRate) "quality citation_unmatched_rate" "value=$($metrics.citation_unmatched_rate) max=$MaxCitationUnmatchedRate"
                }
                Assert-True ([double]$metrics.large_context_pack_rate -le $MaxLargeContextPackRate) "quality large_context_pack_rate" "value=$($metrics.large_context_pack_rate) max=$MaxLargeContextPackRate"
            }

            if ($null -ne $feedbackSummary) {
                Assert-True ([int64]$feedbackSummary.total_feedback -ge $MinFeedbackCount) "quality feedback count" "value=$($feedbackSummary.total_feedback) min=$MinFeedbackCount"
                Assert-True ([int64]$feedbackSummary.matched_cited_source_count -ge $MinMatchedCitedSourceCount) "quality matched cited sources" "value=$($feedbackSummary.matched_cited_source_count) min=$MinMatchedCitedSourceCount"
                if ($historicalQualityDebtAllowed) {
                    Pass "quality unmatched cited sources" "value=$($feedbackSummary.unmatched_cited_source_count) observation=historical_debt_allowed"
                } else {
                    Assert-True ([int64]$feedbackSummary.unmatched_cited_source_count -eq 0) "quality unmatched cited sources" "value=$($feedbackSummary.unmatched_cited_source_count)"
                }
                Assert-True ([int64]$feedbackSummary.missing_context_count -eq 0) "quality missing context count" "value=$($feedbackSummary.missing_context_count)"
                if ($historicalQualityDebtAllowed) {
                    Pass "quality wrong context count" "value=$($feedbackSummary.wrong_context_count) observation=historical_debt_allowed"
                } else {
                    Assert-True ([int64]$feedbackSummary.wrong_context_count -eq 0) "quality wrong context count" "value=$($feedbackSummary.wrong_context_count)"
                }
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

                $nonSyntheticHeaders = if ($ExternalUserId) { Fb2-Headers -UserId $ExternalUserId } else { $fb2Headers }
                $nonSyntheticQuality = Invoke-Json -Url "$Fb2Base/api/main-project/context/quality-summary?$nonSyntheticQuery" -Headers $nonSyntheticHeaders
                $nonSyntheticFeedback = Invoke-Json -Url "$Fb2Base/api/main-project/context/feedback-summary?$nonSyntheticQuery" -Headers $nonSyntheticHeaders
                $nonSyntheticAdoption = Invoke-Json -Url "$Fb2Base/api/main-project/context/opinion-adoption-summary?$nonSyntheticQuery" -Headers $nonSyntheticHeaders

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
$exitFailureCount = [int]$script:Failed
if ($RequireNoSkips -and $script:Skipped -gt 0) {
    Write-Check "FAIL" "no skipped checks" "skipped=$script:Skipped"
    $exitFailureCount += 1
}
if (-not [string]::IsNullOrWhiteSpace($SummaryPath)) {
    $summary = New-Fb2ContractSmokeSummary `
        -Checks $script:Fb2ContractSmokeChecks `
        -FailedCount $exitFailureCount `
        -SkippedCount ([int]$script:Skipped) `
        -MainBase $MainBase `
        -Fb2Base $Fb2Base `
        -GroupId $GroupId `
        -ExternalUserId $ExternalUserId `
        -Fb2TokenPresent (-not [string]::IsNullOrWhiteSpace($Fb2Token)) `
        -RequireFb2Live ([bool]$RequireFb2Live) `
        -RequireNoSkips ([bool]$RequireNoSkips) `
        -SkipVoiceContractChecks ([bool]$SkipVoiceContractChecks)
    Write-Fb2ContractSmokeSummary -Summary $summary -OutputPath $SummaryPath
    Write-Output "OK`tcontract smoke summary`tpath=$SummaryPath complete=$($summary.complete)"
}
if ($exitFailureCount -gt 0) {
    exit 1
}

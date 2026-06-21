#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$MainToken = "",
    [string]$Fb2Base = "",
    [string]$Fb2AiCenterToken = "",
    [string]$Fb2UserToken = "",
    [string]$Fb2Username = "",
    [string]$Fb2Password = "",
    [string]$GroupId = "official",
    [string]$ExternalUserId = "",
    [string]$VoiceDeviceEvidencePath = "",
    [string]$SummaryPath = "",
    [int]$RequestTimeoutSec = 45,
    [int]$PollTimeoutSec = 90,
    [int]$FeedbackPollTimeoutSec = 45,
    [int]$PollIntervalSec = 3,
    [int]$MinFeedbackCount = 2,
    [int]$MinMatchedCitedSourceCount = 2,
    [int]$QualityFeedbackSampleLimit = 10,
    [double]$MaxLargeContextPackRate = 0.75,
    [double]$MaxCitationUnmatchedRate = 0,
    [double]$MaxMissingContextRate = 0,
    [double]$MaxWrongContextRate = 0,
    [switch]$AllowVisibleMessages
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) { $MainBase = $env:ELON_MAIN_BASE }
if (-not $MainBase) { $MainBase = "http://43.139.149.158:8080" }
if (-not $MainToken) { $MainToken = $env:ELON_MAIN_TOKEN }
if (-not $Fb2Base) { $Fb2Base = $env:FB2_API_BASE }
if (-not $Fb2Base) { $Fb2Base = "http://123.207.48.146:8080" }
if (-not $Fb2AiCenterToken) { $Fb2AiCenterToken = $env:FB2_AI_CENTER_TOKEN }
if (-not $Fb2UserToken) { $Fb2UserToken = $env:FB2_USER_TOKEN }
if (-not $Fb2Username) { $Fb2Username = $env:FB2_VISIBLE_SMOKE_USERNAME }
if (-not $Fb2Password) { $Fb2Password = $env:FB2_VISIBLE_SMOKE_PASSWORD }
if (-not $ExternalUserId) { $ExternalUserId = $env:FB2_AI_CONTEXT_EXTERNAL_USER_ID }
if (-not $VoiceDeviceEvidencePath) { $VoiceDeviceEvidencePath = $env:FB2_VOICE_DEVICE_EVIDENCE_PATH }

$MainBase = $MainBase.TrimEnd("/")
$Fb2Base = $Fb2Base.TrimEnd("/")

function Fail-FinalAcceptance {
    param([string]$Message)
    Write-Output "FAIL`tfinal acceptance`t$Message"
    exit 1
}

function Add-Arg {
    param(
        [System.Collections.Generic.List[string]]$Args,
        [string]$Name,
        [object]$Value
    )
    if ($null -ne $Value -and -not [string]::IsNullOrWhiteSpace([string]$Value)) {
        [void]$Args.Add($Name)
        [void]$Args.Add([string]$Value)
    }
}

function Add-SwitchArg {
    param(
        [System.Collections.Generic.List[string]]$Args,
        [string]$Name,
        [bool]$Enabled
    )
    if ($Enabled) {
        [void]$Args.Add($Name)
    }
}

function Invoke-JsonOrNull {
    param([string]$Url)
    try {
        Invoke-RestMethod -Uri $Url -Method Get -TimeoutSec 15
    } catch {
        $null
    }
}

function Invoke-SmokeScript {
    param(
        [string]$Name,
        [System.Collections.Generic.List[string]]$Args
    )

    Write-Output ""
    Write-Output "== $Name =="
    & pwsh @Args
    $exitCode = $LASTEXITCODE
    if ($null -eq $exitCode) { $exitCode = 0 }
    Write-Output "== $Name exit_code=$exitCode =="
    return [int]$exitCode
}

if (-not $AllowVisibleMessages) {
    Fail-FinalAcceptance "Pass -AllowVisibleMessages after explicit authorization; this wrapper sends visible group messages."
}
if (-not $Fb2AiCenterToken) {
    Fail-FinalAcceptance "FB2_AI_CENTER_TOKEN or -Fb2AiCenterToken is required."
}
if (-not $ExternalUserId) {
    Fail-FinalAcceptance "FB2_AI_CONTEXT_EXTERNAL_USER_ID or -ExternalUserId is required for user order verification."
}
if (-not $VoiceDeviceEvidencePath) {
    Fail-FinalAcceptance "FB2_VOICE_DEVICE_EVIDENCE_PATH or -VoiceDeviceEvidencePath is required."
}
if (-not (Test-Path $VoiceDeviceEvidencePath)) {
    Fail-FinalAcceptance "Voice device evidence file not found: $VoiceDeviceEvidencePath"
}
if (-not $MainToken -and -not $Fb2UserToken -and (-not $Fb2Username -or -not $Fb2Password)) {
    Fail-FinalAcceptance "Set ELON_MAIN_TOKEN, FB2_USER_TOKEN, or -Fb2Username/-Fb2Password for authenticated chat flows."
}

$startedAt = (Get-Date).ToUniversalTime().ToString("o")
$qualitySince = $startedAt
$root = Split-Path -Parent $PSScriptRoot
$visibleScript = Join-Path $PSScriptRoot "smoke-fb2-visible-chat.ps1"
$centerScript = Join-Path $PSScriptRoot "smoke-fb2-ai-center.ps1"

$mainHead = ""
try { $mainHead = (& git -C $root rev-parse HEAD).Trim() } catch { $mainHead = "" }
$mainStatus = ""
try { $mainStatus = (& git -C $root status --short --branch) -join "`n" } catch { $mainStatus = "" }
$mainVersion = Invoke-JsonOrNull "$MainBase/api/server/version"
$fb2Version = Invoke-JsonOrNull "$Fb2Base/api/app-version"

$visibleArgs = [System.Collections.Generic.List[string]]::new()
[void]$visibleArgs.Add("-NoProfile")
[void]$visibleArgs.Add("-ExecutionPolicy")
[void]$visibleArgs.Add("Bypass")
[void]$visibleArgs.Add("-File")
[void]$visibleArgs.Add($visibleScript)
Add-Arg $visibleArgs "-MainBase" $MainBase
Add-Arg $visibleArgs "-MainToken" $MainToken
Add-Arg $visibleArgs "-Fb2Base" $Fb2Base
Add-Arg $visibleArgs "-Fb2Token" $Fb2UserToken
Add-Arg $visibleArgs "-Fb2AiCenterToken" $Fb2AiCenterToken
Add-Arg $visibleArgs "-Fb2UserId" $ExternalUserId
Add-Arg $visibleArgs "-Fb2Username" $Fb2Username
Add-Arg $visibleArgs "-Fb2Password" $Fb2Password
Add-Arg $visibleArgs "-GroupId" $GroupId
Add-Arg $visibleArgs "-RequestTimeoutSec" $RequestTimeoutSec
Add-Arg $visibleArgs "-PollTimeoutSec" $PollTimeoutSec
Add-Arg $visibleArgs "-FeedbackPollTimeoutSec" $FeedbackPollTimeoutSec
Add-Arg $visibleArgs "-PollIntervalSec" $PollIntervalSec
Add-SwitchArg $visibleArgs "-AllowVisibleMessages" $true

$visibleExit = Invoke-SmokeScript "visible group chat smoke" $visibleArgs

$centerArgs = [System.Collections.Generic.List[string]]::new()
[void]$centerArgs.Add("-NoProfile")
[void]$centerArgs.Add("-ExecutionPolicy")
[void]$centerArgs.Add("Bypass")
[void]$centerArgs.Add("-File")
[void]$centerArgs.Add($centerScript)
Add-Arg $centerArgs "-MainBase" $MainBase
Add-Arg $centerArgs "-MainToken" $MainToken
Add-Arg $centerArgs "-Fb2Base" $Fb2Base
Add-Arg $centerArgs "-Fb2Token" $Fb2AiCenterToken
Add-Arg $centerArgs "-Fb2UserToken" $Fb2UserToken
Add-Arg $centerArgs "-Fb2Username" $Fb2Username
Add-Arg $centerArgs "-Fb2Password" $Fb2Password
Add-Arg $centerArgs "-GroupId" $GroupId
Add-Arg $centerArgs "-ExternalUserId" $ExternalUserId
Add-Arg $centerArgs "-RequestTimeoutSec" $RequestTimeoutSec
Add-Arg $centerArgs "-VoiceDeviceEvidencePath" $VoiceDeviceEvidencePath
Add-Arg $centerArgs "-QualitySince" $qualitySince
Add-Arg $centerArgs "-MinFeedbackCount" $MinFeedbackCount
Add-Arg $centerArgs "-MinMatchedCitedSourceCount" $MinMatchedCitedSourceCount
Add-Arg $centerArgs "-QualityFeedbackSampleLimit" $QualityFeedbackSampleLimit
Add-Arg $centerArgs "-MaxLargeContextPackRate" $MaxLargeContextPackRate
Add-Arg $centerArgs "-MaxCitationUnmatchedRate" $MaxCitationUnmatchedRate
Add-Arg $centerArgs "-MaxMissingContextRate" $MaxMissingContextRate
Add-Arg $centerArgs "-MaxWrongContextRate" $MaxWrongContextRate
Add-SwitchArg $centerArgs "-FinalAcceptance" $true

$centerExit = Invoke-SmokeScript "final no-skip acceptance" $centerArgs

$completedAt = (Get-Date).ToUniversalTime().ToString("o")
$summary = [ordered]@{
    schema = "fb2.main_project.final_acceptance.v1"
    started_at = $startedAt
    completed_at = $completedAt
    quality_since = $qualitySince
    main_base = $MainBase
    fb2_base = $Fb2Base
    group_id = $GroupId
    external_user_id = $ExternalUserId
    voice_device_evidence_path = $VoiceDeviceEvidencePath
    main_project_head = $mainHead
    main_project_status = $mainStatus
    main_server_version = $mainVersion
    fb2_app_version = $fb2Version
    visible_chat_exit_code = $visibleExit
    final_acceptance_exit_code = $centerExit
    success = ($visibleExit -eq 0 -and $centerExit -eq 0)
}

if (-not $SummaryPath) {
    $summaryDir = Join-Path $root "target\fb2-ai-center"
    New-Item -ItemType Directory -Force -Path $summaryDir | Out-Null
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    $SummaryPath = Join-Path $summaryDir "final-acceptance-$stamp.json"
}

$summaryJson = $summary | ConvertTo-Json -Depth 8
Set-Content -Path $SummaryPath -Value $summaryJson -Encoding UTF8

Write-Output ""
Write-Output "== final acceptance summary =="
Write-Output $summaryJson
Write-Output "summary_path=$SummaryPath"

if (-not $summary.success) {
    exit 1
}

exit 0

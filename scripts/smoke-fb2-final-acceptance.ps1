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
    [switch]$PreflightOnly,
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

function Invoke-Json {
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
        TimeoutSec = 30
    }
    if ($null -ne $Body) {
        $params.ContentType = "application/json"
        $params.Body = ($Body | ConvertTo-Json -Depth 8)
    }
    Invoke-RestMethod @params
}

function Resolve-Fb2ExternalUser {
    if ($ExternalUserId) {
        return
    }
    if (-not $Fb2Username -or -not $Fb2Password) {
        return
    }

    try {
        $login = Invoke-Json -Url "$Fb2Base/api/auth/login" -Method "POST" -Body @{
            username = $Fb2Username
            password = $Fb2Password
        }
    } catch {
        Fail-FinalAcceptance "fb2 login failed before visible smoke: $($_.Exception.Message)"
    }
    if (-not $login.success -or -not $login.data.user.id) {
        Fail-FinalAcceptance "fb2 login did not return a user id."
    }

    $script:ExternalUserId = [string]$login.data.user.id
    if (-not $script:Fb2UserToken -and $login.data.token.access_token) {
        $script:Fb2UserToken = [string]$login.data.token.access_token
    }
    Write-Output "OK`tfinal acceptance external user`t$script:ExternalUserId"
}

function Test-UserOrderContextBeforeVisibleSmoke {
    $topic = [System.Uri]::EscapeDataString("帮我分析我的票")
    $group = [System.Uri]::EscapeDataString($GroupId)
    $user = [System.Uri]::EscapeDataString($ExternalUserId)
    $headers = @{
        "X-FB2-AI-CENTER-TOKEN" = $Fb2AiCenterToken.Trim()
        "X-FB2-AI-CONTEXT-USER-ID" = $ExternalUserId
    }
    $url = "$Fb2Base/api/main-project/context/pack?group_id=$group&external_user_id=$user&topic_hint=$topic&limit=3&order_limit=1"
    try {
        $pack = Invoke-Json -Url $url -Headers $headers
    } catch {
        Fail-FinalAcceptance "fb2 user order context preflight failed before visible smoke: $($_.Exception.Message)"
    }
    if (-not $pack.success) {
        Fail-FinalAcceptance "fb2 user order context preflight failed."
    }
    $orderCount = @($pack.data.user_orders | Where-Object { $_ }).Count
    if ($orderCount -lt 1) {
        Fail-FinalAcceptance "fb2 user order context preflight found no user_orders for ExternalUserId=$ExternalUserId."
    }
    Write-Output "OK`tfinal acceptance user order preflight`torders=$orderCount audit=$($pack.data.context_audit_id)"
}

function Invoke-SmokeScript {
    param(
        [string]$Name,
        [System.Collections.Generic.List[string]]$Args,
        [string]$LogPath
    )

    Write-Output ""
    Write-Output "== $Name =="
    $lines = [System.Collections.Generic.List[string]]::new()
    & pwsh @Args 2>&1 | ForEach-Object {
        $line = [string]$_
        [void]$lines.Add($line)
        Write-Output $line
    }
    $exitCode = $LASTEXITCODE
    if ($null -eq $exitCode) { $exitCode = 0 }
    if ($LogPath) {
        Set-Content -Path $LogPath -Value $lines -Encoding UTF8
    }
    Write-Output "== $Name exit_code=$exitCode =="
    return [pscustomobject]@{
        exit_code = [int]$exitCode
        log_path = $LogPath
        output = @($lines)
    }
}

function Find-CheckDetail {
    param(
        [string[]]$Lines,
        [string]$CheckName
    )

    $prefix = "OK`t$CheckName`t"
    foreach ($line in $Lines) {
        if ($line.StartsWith($prefix)) {
            return $line.Substring($prefix.Length).Trim()
        }
    }
    return ""
}

function Find-FeedbackEvidence {
    param([string[]]$Lines)

    $items = @()
    foreach ($line in $Lines) {
        if ($line -match '^OK\t(?<scenario>.+ fb2 feedback)\t(?<request>\S+) feedback=(?<feedback>\S+)') {
            $items += [pscustomobject]@{
                scenario = $Matches.scenario
                main_request_id = $Matches.request
                feedback_id = $Matches.feedback
            }
        }
    }
    return $items
}

if ($PreflightOnly -and $AllowVisibleMessages) {
    Fail-FinalAcceptance "Use either -PreflightOnly or -AllowVisibleMessages, not both."
}
if (-not $PreflightOnly -and -not $AllowVisibleMessages) {
    Fail-FinalAcceptance "Pass -AllowVisibleMessages after explicit authorization; this wrapper sends visible group messages."
}
if (-not $Fb2AiCenterToken) {
    Fail-FinalAcceptance "FB2_AI_CENTER_TOKEN or -Fb2AiCenterToken is required."
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
if (-not $ExternalUserId -and (-not $Fb2Username -or -not $Fb2Password)) {
    Fail-FinalAcceptance "Set FB2_AI_CONTEXT_EXTERNAL_USER_ID or provide -Fb2Username/-Fb2Password so the wrapper can resolve the fb2 user id."
}

Resolve-Fb2ExternalUser
if (-not $ExternalUserId) {
    Fail-FinalAcceptance "Unable to resolve fb2 external user id from credentials; pass -ExternalUserId explicitly."
}
Test-UserOrderContextBeforeVisibleSmoke

$startedAt = (Get-Date).ToUniversalTime().ToString("o")
$qualitySince = $startedAt
$root = Split-Path -Parent $PSScriptRoot
$visibleScript = Join-Path $PSScriptRoot "smoke-fb2-visible-chat.ps1"
$centerScript = Join-Path $PSScriptRoot "smoke-fb2-ai-center.ps1"
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

if (-not $SummaryPath) {
    $summaryDir = Join-Path $root "target\fb2-ai-center"
    $SummaryPath = Join-Path $summaryDir "final-acceptance-$stamp.json"
} else {
    $summaryDir = Split-Path -Parent $SummaryPath
    if (-not $summaryDir) {
        $summaryDir = "."
    }
}
New-Item -ItemType Directory -Force -Path $summaryDir | Out-Null
$visibleLogPath = Join-Path $summaryDir "final-acceptance-$stamp-visible-chat.log"
$centerLogPath = Join-Path $summaryDir "final-acceptance-$stamp-ai-center.log"

$mainHead = ""
try { $mainHead = (& git -C $root rev-parse HEAD).Trim() } catch { $mainHead = "" }
$mainStatus = ""
try { $mainStatus = (& git -C $root status --short --branch) -join "`n" } catch { $mainStatus = "" }
$mainVersion = Invoke-JsonOrNull "$MainBase/api/server/version"
$fb2Version = Invoke-JsonOrNull "$Fb2Base/api/app-version"

if ($PreflightOnly) {
    $preflightArgs = [System.Collections.Generic.List[string]]::new()
    [void]$preflightArgs.Add("-NoProfile")
    [void]$preflightArgs.Add("-ExecutionPolicy")
    [void]$preflightArgs.Add("Bypass")
    [void]$preflightArgs.Add("-File")
    [void]$preflightArgs.Add($centerScript)
    Add-Arg $preflightArgs "-MainBase" $MainBase
    Add-Arg $preflightArgs "-MainToken" $MainToken
    Add-Arg $preflightArgs "-Fb2Base" $Fb2Base
    Add-Arg $preflightArgs "-Fb2Token" $Fb2AiCenterToken
    Add-Arg $preflightArgs "-Fb2UserToken" $Fb2UserToken
    Add-Arg $preflightArgs "-Fb2Username" $Fb2Username
    Add-Arg $preflightArgs "-Fb2Password" $Fb2Password
    Add-Arg $preflightArgs "-GroupId" $GroupId
    Add-Arg $preflightArgs "-ExternalUserId" $ExternalUserId
    Add-Arg $preflightArgs "-RequestTimeoutSec" $RequestTimeoutSec
    Add-Arg $preflightArgs "-VoiceDeviceEvidencePath" $VoiceDeviceEvidencePath
    Add-SwitchArg $preflightArgs "-RequireFb2Live" $true
    Add-SwitchArg $preflightArgs "-RequireAllScenarios" $true
    Add-SwitchArg $preflightArgs "-IncludePlatformOrderSummary" $true
    Add-SwitchArg $preflightArgs "-CheckFb2ApkVersion" $true
    Add-SwitchArg $preflightArgs "-CheckLocalVoiceSdkBuild" $true
    Add-SwitchArg $preflightArgs "-RequireVoiceDeviceEvidence" $true
    Add-SwitchArg $preflightArgs "-RequireNoSkips" $true

    $preflightLogPath = Join-Path $summaryDir "final-acceptance-$stamp-preflight.log"
    $preflightResult = Invoke-SmokeScript "final preflight without visible messages" $preflightArgs $preflightLogPath
    $completedAt = (Get-Date).ToUniversalTime().ToString("o")
    $summary = [ordered]@{
        schema = "fb2.main_project.final_acceptance.v1"
        mode = "preflight_only"
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
        preflight_exit_code = $preflightResult.exit_code
        preflight_log_path = $preflightResult.log_path
        success = ($preflightResult.exit_code -eq 0)
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
}

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

$visibleResult = Invoke-SmokeScript "visible group chat smoke" $visibleArgs $visibleLogPath
$visibleLines = @($visibleResult.output)

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

$centerResult = Invoke-SmokeScript "final no-skip acceptance" $centerArgs $centerLogPath

$completedAt = (Get-Date).ToUniversalTime().ToString("o")
$feedbackEvidence = @(Find-FeedbackEvidence $visibleLines)
$summary = [ordered]@{
    schema = "fb2.main_project.final_acceptance.v1"
    mode = "visible_final_acceptance"
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
    visible_chat_exit_code = $visibleResult.exit_code
    final_acceptance_exit_code = $centerResult.exit_code
    visible_chat_log_path = $visibleResult.log_path
    final_acceptance_log_path = $centerResult.log_path
    visible_mention_message_id = Find-CheckDetail $visibleLines "visible @EL sent"
    visible_mention_reply_id = Find-CheckDetail $visibleLines "visible @EL ai reply"
    selected_message_seed_id = Find-CheckDetail $visibleLines "selected-message seed sent"
    selected_message_reply_id = Find-CheckDetail $visibleLines "selected-message ai reply"
    feedback_evidence = $feedbackEvidence
    success = ($visibleResult.exit_code -eq 0 -and $centerResult.exit_code -eq 0)
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

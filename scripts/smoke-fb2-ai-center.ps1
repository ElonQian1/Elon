#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$MainToken = "",
    [string]$Fb2Base = "",
    [string]$Fb2Token = "",
    [string]$GroupId = "official",
    [string]$ExternalUserId = "",
    [int]$RequestTimeoutSec = 45,
    [int]$RetryCount = 1,
    [switch]$IncludePlatformOrderSummary,
    [switch]$RequireFb2Live,
    [switch]$RequireAllScenarios,
    [switch]$CheckQuality,
    [switch]$RequireFeedbackCoverage,
    [string]$QualitySince = "",
    [string]$QualityUntil = "",
    [double]$MaxLargeContextPackRate = 0.75,
    [double]$MaxCitationUnmatchedRate = 0,
    [double]$MaxMissingContextRate = 0,
    [double]$MaxWrongContextRate = 0,
    [int]$MinFeedbackCount = 0,
    [int]$MinMatchedCitedSourceCount = 0,
    [int]$QualityFeedbackSampleLimit = 5
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

$MainBase = $MainBase.TrimEnd("/")
$Fb2Base = $Fb2Base.TrimEnd("/")
$amp = [char]38
$script:Failed = 0
$script:Skipped = 0

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
    -or $PSBoundParameters.ContainsKey("MinMatchedCitedSourceCount")

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
        Assert-True ([bool]$bootstrap.voice.composer) "chat-bootstrap voice composer"
        Assert-True ([bool]$bootstrap.billing) "chat-bootstrap billing"
    } catch {
        Fail "chat-bootstrap" $_.Exception.Message
    }
} else {
    Skip "chat-bootstrap" "set ELON_MAIN_TOKEN or -MainToken to verify authenticated bootstrap"
}

try {
    $contract = Invoke-Json "$MainBase/api/external/apps/fb2/context-contract"
    $policy = $contract.live_tool_manifest.main_project_tool_execution_policy
    Assert-True ($contract.live_tool_manifest.status -eq "ready") "live manifest ready" "tool_count=$($contract.live_tool_manifest.tool_count)"
    Assert-True ($policy.schema -eq "external_app.live_tool_execution_policy.v1") "live manifest execution policy"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "search_matches") -and ($policy.chat_auto_executable_tool_ids -contains "search_group_opinions")) "auto executable core tools"
    Assert-True (($policy.chat_auto_executable_tool_ids -contains "match_analysis_brief") -and ($policy.chat_auto_executable_tool_ids -contains "group_opinion_summary")) "auto executable aggregate tools"
    Assert-True ($policy.manifest_only_tool_ids -contains "record_context_feedback") "callback tool is not chat-auto-executable"
    Assert-True (@($policy.main_project_allowed_missing_tool_ids).Count -eq 0) "no allowed tool missing in live fb2 manifest"
} catch {
    Fail "context-contract" $_.Exception.Message
}

if (-not $Fb2Token) {
    if ($RequireFb2Live -or $RequireAllScenarios -or $qualityCheckRequested) {
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
        } catch {
            Fail "fb2 quality gates" $_.Exception.Message
        }
    }
}

Write-Output ""
Write-Output "== Summary =="
Write-Output "failed=$script:Failed skipped=$script:Skipped"
if ($script:Failed -gt 0) {
    exit 1
}

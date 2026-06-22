#requires -Version 7.0

param(
    [string]$InputPath = "",
    [string]$Scenario = "custom",
    [string[]]$ExpectedSourceKinds = @(),
    [switch]$PrintExportRequest,
    [string]$OutputPath = "",
    [string]$GroupId = "official",
    [string]$ExternalUserId = "",
    [string]$TopicHint = "今天比赛怎么看",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"
$script:Failed = 0

function Write-CheckOk {
    param(
        [string]$Name,
        [string]$Detail = ""
    )

    if ([string]::IsNullOrWhiteSpace($Detail)) {
        Write-Output "OK`t$Name"
    } else {
        Write-Output "OK`t$Name`t$Detail"
    }
}

function Write-CheckFail {
    param(
        [string]$Name,
        [string]$Detail = ""
    )

    $script:Failed += 1
    if ([string]::IsNullOrWhiteSpace($Detail)) {
        Write-Output "FAIL`t$Name"
    } else {
        Write-Output "FAIL`t$Name`t$Detail"
    }
}

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Name,
        [string]$Detail = ""
    )

    if ($Condition) {
        Write-CheckOk $Name $Detail
    } else {
        Write-CheckFail $Name $Detail
    }
}

function Assert-ContainsValue {
    param(
        [object[]]$Values,
        [string]$Expected,
        [string]$Name
    )

    $normalized = @($Values | ForEach-Object { [string]$_ })
    Assert-True ($normalized -contains $Expected) $Name ($normalized -join ";")
}

. (Join-Path $PSScriptRoot "fb2-ai-center-context-projection.ps1")

function Get-DefaultExpectedSourceKinds {
    param([string]$ScenarioName)

    switch ($ScenarioName.ToLowerInvariant()) {
        "today" { return @("match", "odds", "context_audit") }
        "today_matches" { return @("match", "odds", "context_audit") }
        "today_matches_context_pack" { return @("match", "odds", "context_audit") }
        "my_ticket" { return @("user_order", "ticket", "context_audit") }
        "my_ticket_context_pack" { return @("user_order", "ticket", "context_audit") }
        "platform_order" { return @("platform_order_summary", "context_audit") }
        "platform_order_context_pack" { return @("platform_order_summary", "context_audit") }
        "group_opinion" { return @("group_message", "opinion_memory", "context_audit") }
        "group_opinion_context_pack" { return @("group_message", "opinion_memory", "context_audit") }
        default { return @() }
    }
}

function Normalize-Fb2ContextPackInput {
    param([object]$Payload)

    if ($null -eq $Payload) {
        return $null
    }
    if ($Payload.PSObject.Properties["data"] -and $Payload.data.PSObject.Properties["context_pack"]) {
        return $Payload.data
    }
    if ($Payload.PSObject.Properties["context_pack"]) {
        return $Payload
    }
    return $Payload
}

function New-Fb2ContextPackSampleScenario {
    param(
        [string]$Id,
        [string]$Question,
        [string]$Path,
        [hashtable]$Headers,
        [string[]]$ExpectedKinds
    )

    [ordered]@{
        id = $Id
        user_question = $Question
        method = "GET"
        path = $Path
        required_headers = $Headers
        save_as = "target/fb2-ai-center/samples/$Id.json"
        expected_source_kinds = @($ExpectedKinds)
        validate_command = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -InputPath target\fb2-ai-center\samples\$Id.json -Scenario $Id"
    }
}

function New-Fb2ContextPackSampleRequest {
    param(
        [string]$Group,
        [string]$UserId,
        [string]$Hint
    )

    $effectiveUserId = if ([string]::IsNullOrWhiteSpace($UserId)) { "<fb2_user_uuid_with_orders>" } else { $UserId }
    $encodedHint = [System.Uri]::EscapeDataString($Hint)
    $ticketHint = [System.Uri]::EscapeDataString("帮我分析我的票")
    $platformHint = [System.Uri]::EscapeDataString("平台今天订单风险怎么样")
    $opinionHint = [System.Uri]::EscapeDataString("群里大家怎么看这场")

    $scenarios = @(
        New-Fb2ContextPackSampleScenario `
            -Id "today_matches_context_pack" `
            -Question $Hint `
            -Path "/api/main-project/context/pack?group_id=$Group&topic_hint=$encodedHint&limit=10&discussion_limit=20" `
            -Headers @{ "X-FB2-AI-CENTER-TOKEN" = "<service-token>" } `
            -ExpectedKinds @("match", "odds", "context_audit")
        New-Fb2ContextPackSampleScenario `
            -Id "my_ticket_context_pack" `
            -Question "帮我分析我的票" `
            -Path "/api/main-project/context/pack?group_id=$Group&external_user_id=$effectiveUserId&topic_hint=$ticketHint&limit=10&order_limit=10" `
            -Headers @{
                "X-FB2-AI-CENTER-TOKEN" = "<service-token>"
                "X-FB2-AI-CONTEXT-USER-ID" = $effectiveUserId
            } `
            -ExpectedKinds @("user_order", "ticket", "context_audit")
        New-Fb2ContextPackSampleScenario `
            -Id "platform_order_context_pack" `
            -Question "平台今天订单风险怎么样" `
            -Path "/api/main-project/context/pack?group_id=$Group&topic_hint=$platformHint&include_platform_orders=true&limit=10" `
            -Headers @{
                "X-FB2-AI-CENTER-TOKEN" = "<service-token>"
                "X-FB2-AI-CONTEXT-SCOPE" = "platform_order_summary"
            } `
            -ExpectedKinds @("platform_order_summary", "context_audit")
        New-Fb2ContextPackSampleScenario `
            -Id "group_opinion_context_pack" `
            -Question "群里大家怎么看这场" `
            -Path "/api/main-project/context/pack?group_id=$Group&topic_hint=$opinionHint&limit=10&discussion_limit=30" `
            -Headers @{ "X-FB2-AI-CENTER-TOKEN" = "<service-token>" } `
            -ExpectedKinds @("group_message", "opinion_memory", "context_audit")
    )

    [ordered]@{
        schema = "fb2.main_project.context_pack_sample_request.v1"
        purpose = "Ask fb2 to export live Context Pack samples for main-project offline validation when FB2_AI_CENTER_TOKEN is not available in the main-project session."
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        fb2_base = "<fb2-api-base>"
        group_id = $Group
        external_user_id = $effectiveUserId
        scenarios = @($scenarios)
        response_file_format = @{
            accepted_shapes = @("full_http_response_with_data", "data_object_with_context_pack")
            required_fields = @("context_pack", "context_audit_id", "citation_sources")
            must_include_sections = @("identity", "data_summary", "matches", "odds", "user_orders", "group_opinions", "retrieval_evidence")
        }
        redaction_rules = @(
            "Do not include service tokens in saved files, commits, logs, or chat messages.",
            "Do not include raw message bodies outside the Context Pack sample; hashes and source ids are preferred for handoff.",
            "User order samples must belong to external_user_id and must not expose other users' individual orders.",
            "Platform order summary must remain anonymous aggregate only."
        )
        main_project_validation = @{
            command_template = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -InputPath <sample-json> -Scenario <scenario-id>"
            offline_only = $true
            does_not_write_group = $true
        }
    }
}

function Write-Fb2ContextPackSampleRequest {
    param(
        [string]$Path,
        [object]$Request
    )

    $json = $Request | ConvertTo-Json -Depth 12
    if ([string]::IsNullOrWhiteSpace($Path)) {
        Write-Output $json
        return
    }

    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -LiteralPath $Path -Value $json -Encoding UTF8
    Write-CheckOk "export request written" $Path
}

function Invoke-Fb2ContextPackFileValidation {
    param(
        [string]$Path,
        [string]$ScenarioName,
        [string[]]$ExpectedKinds
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        Write-CheckFail "input path" "Pass -InputPath <context-pack.json>"
        return
    }
    if (-not (Test-Path -LiteralPath $Path)) {
        Write-CheckFail "input path" "file not found: $Path"
        return
    }

    try {
        $payload = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    } catch {
        Write-CheckFail "input json" $_.Exception.Message
        return
    }

    $data = Normalize-Fb2ContextPackInput $payload
    if ($ExpectedKinds.Count -eq 0) {
        $ExpectedKinds = @(Get-DefaultExpectedSourceKinds $ScenarioName)
    }

    Assert-Fb2ContextPackProjection -Data $data -Scenario $ScenarioName -ExpectedSourceKinds $ExpectedKinds
}

function Invoke-Fb2ContextPackValidatorSelfTest {
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-context-pack-validator-{0}" -f ([guid]::NewGuid().ToString("N")))
    New-Item -ItemType Directory -Path $tmp -Force | Out-Null
    try {
        $valid = [ordered]@{
            success = $true
            data = New-Fb2ContextProjectionSelfTestData
        }
        $validPath = Join-Path $tmp "valid.json"
        $valid | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $validPath -Encoding UTF8

        $beforeValid = $script:Failed
        Invoke-Fb2ContextPackFileValidation -Path $validPath -ScenarioName "today_matches_context_pack" -ExpectedKinds @("match", "odds", "context_audit")
        $validFailures = $script:Failed - $beforeValid
        if ($validFailures -eq 0) {
            Write-Output "OK`tself-test valid context pack"
        } else {
            Write-Output "FAIL`tself-test valid context pack`tcase_failures=$validFailures"
        }

        $bad = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $bad.data.context_pack = $bad.data.context_pack.Replace("## retrieval_evidence 召回理由和数据缺口", "## other_section")
        $badPath = Join-Path $tmp "missing-section.json"
        $bad | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $badPath -Encoding UTF8

        $beforeBad = $script:Failed
        $null = @(Invoke-Fb2ContextPackFileValidation -Path $badPath -ScenarioName "today_matches_context_pack" -ExpectedKinds @("match"))
        $badFailures = $script:Failed - $beforeBad
        $script:Failed = $beforeBad
        if ($badFailures -gt 0) {
            Write-Output "OK`tself-test rejects missing retrieval_evidence`tcase_failures=$badFailures"
        } else {
            $script:Failed += 1
            Write-Output "FAIL`tself-test rejects missing retrieval_evidence"
        }

        $request = New-Fb2ContextPackSampleRequest -Group "official" -UserId "6fe5aa17-0403-427a-8e91-7f414beca35d" -Hint "今天比赛怎么看"
        Assert-True ([string]$request.schema -eq "fb2.main_project.context_pack_sample_request.v1") "self-test export request schema" ([string]$request.schema)
        Assert-True (@($request.scenarios).Count -eq 4) "self-test export request scenario count" "count=$(@($request.scenarios).Count)"
        $scenarioIds = @($request.scenarios | ForEach-Object { [string]$_["id"] })
        foreach ($expected in @("today_matches_context_pack", "my_ticket_context_pack", "platform_order_context_pack", "group_opinion_context_pack")) {
            Assert-ContainsValue -Values $scenarioIds -Expected $expected -Name "self-test export request includes $expected"
        }
        $requestJson = $request | ConvertTo-Json -Depth 12
        Assert-True ($requestJson -notmatch "123qwe|Bearer\s+\S+") "self-test export request does not leak secrets"
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-Fb2ContextPackValidatorSelfTest
} elseif ($PrintExportRequest) {
    $request = New-Fb2ContextPackSampleRequest -Group $GroupId -UserId $ExternalUserId -Hint $TopicHint
    Write-Fb2ContextPackSampleRequest -Path $OutputPath -Request $request
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        if ($script:Failed -gt 0) {
            exit 1
        }
        exit 0
    }
} else {
    Invoke-Fb2ContextPackFileValidation -Path $InputPath -ScenarioName $Scenario -ExpectedKinds $ExpectedSourceKinds
}

Write-Output ""
Write-Output "== Summary =="
Write-Output "failed=$script:Failed"
if ($script:Failed -gt 0) {
    exit 1
}

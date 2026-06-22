#requires -Version 7.0

param(
    [string]$InputPath = "",
    [string]$Scenario = "custom",
    [string[]]$ExpectedSourceKinds = @(),
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
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-Fb2ContextPackValidatorSelfTest
} else {
    Invoke-Fb2ContextPackFileValidation -Path $InputPath -ScenarioName $Scenario -ExpectedKinds $ExpectedSourceKinds
}

Write-Output ""
Write-Output "== Summary =="
Write-Output "failed=$script:Failed"
if ($script:Failed -gt 0) {
    exit 1
}

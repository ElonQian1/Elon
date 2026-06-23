#requires -Version 7.0

param(
    [string]$SummaryPath = "",
    [string]$OutputPath = "",
    [int]$MaxContextPackChars = 24000,
    [int]$TargetContextPackChars = 12000,
    [int]$MinCitationSources = 2,
    [int]$MaxCitationSources = 60,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2BudgetRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2BudgetPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Get-Fb2BudgetProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    if ($Object -is [System.Collections.IDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    if ($Object -is [System.Collections.Specialized.OrderedDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Get-DefaultFb2BudgetSummaryPath {
    param([string]$Root)

    $targetDir = Join-Path $Root "target\fb2-ai-center"
    $preferred = Join-Path $targetDir "fb2-repo-context-pack-samples-validation-current.json"
    if (Test-Path -LiteralPath $preferred) {
        return $preferred
    }
    $fallback = Join-Path $targetDir "context-pack-samples-validation-current.json"
    if (Test-Path -LiteralPath $fallback) {
        return $fallback
    }
    return ""
}

function Read-Fb2BudgetJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Context Pack sample validation summary not found: $Path"
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2BudgetCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        details = $Details
    })
}

function Add-Fb2BudgetWarning {
    param(
        [System.Collections.ArrayList]$Warnings,
        [string]$Name,
        [string]$Details = ""
    )

    [void]$Warnings.Add([ordered]@{
        name = $Name
        details = $Details
    })
}

function Get-Fb2BudgetExpectedScenarios {
    @(
        "today_matches_context_pack",
        "my_ticket_context_pack",
        "platform_order_context_pack",
        "group_opinion_context_pack"
    )
}

function New-Fb2ContextPackBudgetValidation {
    param(
        [object]$Summary,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $warnings = [System.Collections.ArrayList]::new()
    $scenarios = @((Get-Fb2BudgetProperty $Summary "scenarios" @()))
    $scenarioIds = @($scenarios | ForEach-Object { [string](Get-Fb2BudgetProperty $_ "scenario" "") })
    $scenarioMetrics = [System.Collections.ArrayList]::new()

    Add-Fb2BudgetCheck $checks "summary schema" ([string](Get-Fb2BudgetProperty $Summary "schema" "") -eq "fb2.main_project.context_pack_sample_set_validation.v1") ([string](Get-Fb2BudgetProperty $Summary "schema" ""))
    Add-Fb2BudgetCheck $checks "summary complete" ([bool](Get-Fb2BudgetProperty $Summary "complete" $false))
    Add-Fb2BudgetCheck $checks "summary failed count zero" ([int](Get-Fb2BudgetProperty $Summary "failed_count" 0) -eq 0) "failed_count=$([int](Get-Fb2BudgetProperty $Summary "failed_count" 0))"

    foreach ($expected in Get-Fb2BudgetExpectedScenarios) {
        Add-Fb2BudgetCheck $checks "scenario present: $expected" ($scenarioIds -contains $expected)
    }

    foreach ($scenario in $scenarios) {
        $id = [string](Get-Fb2BudgetProperty $scenario "scenario" "")
        if ([string]::IsNullOrWhiteSpace($id)) {
            Add-Fb2BudgetCheck $checks "scenario id present" $false
            continue
        }

        $chars = [int](Get-Fb2BudgetProperty $scenario "context_pack_chars" 0)
        $citationCount = [int](Get-Fb2BudgetProperty $scenario "citation_source_count" 0)
        $sha = [string](Get-Fb2BudgetProperty $scenario "context_pack_sha256" "")
        $passed = [bool](Get-Fb2BudgetProperty $scenario "passed" $false)

        Add-Fb2BudgetCheck $checks "$id validation passed" $passed
        Add-Fb2BudgetCheck $checks "$id context chars present" ($chars -gt 0) "chars=$chars"
        Add-Fb2BudgetCheck $checks "$id context chars hard budget" ($chars -le $MaxContextPackChars) "chars=$chars max=$MaxContextPackChars"
        Add-Fb2BudgetCheck $checks "$id citation count lower bound" ($citationCount -ge $MinCitationSources) "count=$citationCount min=$MinCitationSources"
        Add-Fb2BudgetCheck $checks "$id citation count upper bound" ($citationCount -le $MaxCitationSources) "count=$citationCount max=$MaxCitationSources"
        Add-Fb2BudgetCheck $checks "$id context pack sha256" ($sha -match '^[a-f0-9]{64}$') $sha

        if ($chars -gt $TargetContextPackChars) {
            Add-Fb2BudgetWarning $warnings "$id exceeds target context budget" "chars=$chars target=$TargetContextPackChars hard_max=$MaxContextPackChars"
        }

        [void]$scenarioMetrics.Add([ordered]@{
            scenario = $id
            context_pack_chars = $chars
            target_context_pack_chars = $TargetContextPackChars
            max_context_pack_chars = $MaxContextPackChars
            citation_source_count = $citationCount
            max_citation_sources = $MaxCitationSources
            context_pack_sha256 = $sha
            over_target = ($chars -gt $TargetContextPackChars)
            over_hard_budget = ($chars -gt $MaxContextPackChars)
        })
    }

    $maxChars = 0
    foreach ($metric in $scenarioMetrics) {
        $metricChars = [int](Get-Fb2BudgetProperty $metric "context_pack_chars" 0)
        if ($metricChars -gt $maxChars) {
            $maxChars = $metricChars
        }
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.context_pack_budget_validation.v1"
        source_summary = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        warning_count = @($warnings).Count
        failed = @($failed)
        warnings = @($warnings)
        checks = @($checks)
        metrics = [ordered]@{
            scenario_count = @($scenarioMetrics).Count
            max_context_pack_chars_observed = $maxChars
            target_context_pack_chars = $TargetContextPackChars
            max_context_pack_chars = $MaxContextPackChars
            min_citation_sources = $MinCitationSources
            max_citation_sources = $MaxCitationSources
            scenarios = @($scenarioMetrics)
        }
        note = "Validates exported fb2 Context Pack sample budget using only sample validation summaries. The hard budget prevents large prompts from regressing; target budget warnings guide future compression toward faster retrieval."
    }
}

function New-Fb2BudgetFixture {
    param(
        [int]$TodayChars = 11000,
        [int]$TicketChars = 18000,
        [int]$PlatformChars = 11500,
        [int]$OpinionChars = 11600,
        [bool]$Complete = $true,
        [int]$CitationCount = 12
    )

    $hash = "a" * 64
    [pscustomobject][ordered]@{
        schema = "fb2.main_project.context_pack_sample_set_validation.v1"
        complete = $Complete
        failed_count = if ($Complete) { 0 } else { 1 }
        scenarios = @(
            [pscustomobject]@{ scenario = "today_matches_context_pack"; passed = $true; context_pack_chars = $TodayChars; citation_source_count = $CitationCount; context_pack_sha256 = $hash },
            [pscustomobject]@{ scenario = "my_ticket_context_pack"; passed = $true; context_pack_chars = $TicketChars; citation_source_count = $CitationCount; context_pack_sha256 = $hash },
            [pscustomobject]@{ scenario = "platform_order_context_pack"; passed = $true; context_pack_chars = $PlatformChars; citation_source_count = $CitationCount; context_pack_sha256 = $hash },
            [pscustomobject]@{ scenario = "group_opinion_context_pack"; passed = $true; context_pack_chars = $OpinionChars; citation_source_count = $CitationCount; context_pack_sha256 = $hash }
        )
    }
}

function Invoke-Fb2BudgetSelfTest {
    $failed = 0
    $good = New-Fb2BudgetFixture
    $goodResult = New-Fb2ContextPackBudgetValidation -Summary $good -SourcePath "selftest-good.json"
    if (-not [bool]$goodResult.success) {
        $goodResult | ConvertTo-Json -Depth 8
        $failed++
    }
    if ([int]$goodResult.warning_count -lt 1) {
        $failed++
    }

    $large = New-Fb2BudgetFixture -TicketChars 25000
    $largeResult = New-Fb2ContextPackBudgetValidation -Summary $large -SourcePath "selftest-large.json"
    if ([bool]$largeResult.success) {
        $failed++
    }

    $fewSources = New-Fb2BudgetFixture -CitationCount 1
    $fewSourcesResult = New-Fb2ContextPackBudgetValidation -Summary $fewSources -SourcePath "selftest-few-sources.json"
    if ([bool]$fewSourcesResult.success) {
        $failed++
    }

    $incomplete = New-Fb2BudgetFixture -Complete:$false
    $incompleteResult = New-Fb2ContextPackBudgetValidation -Summary $incomplete -SourcePath "selftest-incomplete.json"
    if ([bool]$incompleteResult.success) {
        $failed++
    }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2BudgetSelfTest
    exit 0
}

$root = Get-Fb2BudgetRepoRoot
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Get-DefaultFb2BudgetSummaryPath -Root $root
} else {
    $SummaryPath = Resolve-Fb2BudgetPath -Path $SummaryPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    throw "No Context Pack sample validation summary found under target\fb2-ai-center."
}

$summary = Read-Fb2BudgetJson -Path $SummaryPath
$result = New-Fb2ContextPackBudgetValidation -Summary $summary -SourcePath $SummaryPath

if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Resolve-Fb2BudgetPath -Path $OutputPath -Root $root
    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
}

$result | ConvertTo-Json -Depth 8
if (-not [bool]$result.success) {
    exit 1
}

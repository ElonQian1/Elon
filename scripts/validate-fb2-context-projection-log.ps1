#requires -Version 7.0

param(
    [string]$StatusPath = "",
    [string]$LogPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-context-projection-log-validation.ps1")

function Get-Fb2ProjectionValidationRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2ProjectionValidationPath {
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

function Get-Fb2ProjectionValidationProperty {
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
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Read-Fb2ProjectionValidationJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2ProjectionValidationCheck {
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

function Test-Fb2ProjectionValidationContainsAll {
    param(
        [string[]]$Values,
        [string[]]$Required
    )

    foreach ($item in $Required) {
        if (-not ($Values -contains $item)) {
            return $false
        }
    }
    return $true
}

function New-Fb2ProjectionValidationResult {
    param(
        [object]$Status,
        [string]$StatusPath,
        [string]$LogPath
    )

    $checks = [System.Collections.ArrayList]::new()
    $statusProjection = Get-Fb2ProjectionValidationProperty $Status "latest_ai_center_context_projection"
    if ([string]::IsNullOrWhiteSpace($LogPath)) {
        $LogPath = [string](Get-Fb2ProjectionValidationProperty $statusProjection "path" "")
    }
    $state = Get-Fb2ContextProjectionLogState -Path $LogPath
    $today = Get-Fb2ProjectionValidationProperty $state "today_matches_context_pack"
    $ticket = Get-Fb2ProjectionValidationProperty $state "my_ticket_context_pack"
    $business = Get-Fb2ProjectionValidationProperty $state "business_data_checks"
    $statusComplete = if ($null -eq $statusProjection) { $null } else { [bool](Get-Fb2ProjectionValidationProperty $statusProjection "complete" $false) }
    $statusPathValue = [string](Get-Fb2ProjectionValidationProperty $statusProjection "path" "")

    Add-Fb2ProjectionValidationCheck $checks "log path present" (-not [string]::IsNullOrWhiteSpace($LogPath)) $LogPath
    Add-Fb2ProjectionValidationCheck $checks "log exists" ([bool](Get-Fb2ProjectionValidationProperty $state "exists" $false)) $LogPath
    Add-Fb2ProjectionValidationCheck $checks "projection complete" ([bool](Get-Fb2ProjectionValidationProperty $state "complete" $false))
    Add-Fb2ProjectionValidationCheck $checks "projection missing list empty" (@(Get-Fb2ProjectionValidationProperty $state "missing" @()).Count -eq 0)
    Add-Fb2ProjectionValidationCheck $checks "today matches context pack complete" ([bool](Get-Fb2ProjectionValidationProperty $today "complete" $false))
    Add-Fb2ProjectionValidationCheck $checks "today matches source kinds" (Test-Fb2ProjectionValidationContainsAll -Values @((Get-Fb2ProjectionValidationProperty $today "expected_source_kinds" @()) | ForEach-Object { [string]$_ }) -Required @("match", "odds", "context_audit"))
    Add-Fb2ProjectionValidationCheck $checks "my ticket context pack complete" ([bool](Get-Fb2ProjectionValidationProperty $ticket "complete" $false))
    Add-Fb2ProjectionValidationCheck $checks "my ticket source kinds" (Test-Fb2ProjectionValidationContainsAll -Values @((Get-Fb2ProjectionValidationProperty $ticket "expected_source_kinds" @()) | ForEach-Object { [string]$_ }) -Required @("user_order", "ticket", "context_audit"))

    foreach ($name in @("group_opinion_summary", "platform_order_summary", "quality_unmatched_sources_zero", "non_synthetic_opinion_adoption")) {
        Add-Fb2ProjectionValidationCheck $checks "business data check $name" ([bool](Get-Fb2ProjectionValidationProperty $business $name $false))
    }

    if ($null -ne $Status) {
        Add-Fb2ProjectionValidationCheck $checks "status projection present" ($null -ne $statusProjection)
        Add-Fb2ProjectionValidationCheck $checks "status projection complete matches log" ($statusComplete -eq [bool](Get-Fb2ProjectionValidationProperty $state "complete" $false))
        Add-Fb2ProjectionValidationCheck $checks "status projection path matches log" (
            [string]::IsNullOrWhiteSpace($statusPathValue) -or
            [string]::Equals($statusPathValue, [string]$LogPath, [System.StringComparison]::OrdinalIgnoreCase)
        )
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.context_projection_log_validation.v1"
        source_status = $StatusPath
        source_log = $LogPath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        projection = $state
        note = "Validates local AI Center context projection evidence only; protected live refresh still requires FB2_AI_CENTER_TOKEN."
    }
}

function New-Fb2ProjectionValidationSelfTestLog {
    param(
        [string]$Path,
        [switch]$MissingPlatform
    )

    $checks = @(
        "context projection body: today matches context pack",
        "context projection wrapper open: today matches context pack",
        "context projection wrapper close: today matches context pack",
        "context projection audit id: today matches context pack",
        "context projection source registry: today matches context pack",
        "context projection section: today matches context pack/usage_boundary",
        "context projection section: today matches context pack/match_facts",
        "context projection section: today matches context pack/user_order_slice",
        "context projection section: today matches context pack/platform_order_summary",
        "context projection section: today matches context pack/group_opinion_slice",
        "context projection section: today matches context pack/retrieval_evidence",
        "context projection section: today matches context pack/quality_feedback",
        "context projection source kind: today matches context pack/match",
        "context projection source kind: today matches context pack/odds",
        "context projection source kind: today matches context pack/context_audit",
        "context projection body: my ticket context pack",
        "context projection wrapper open: my ticket context pack",
        "context projection wrapper close: my ticket context pack",
        "context projection audit id: my ticket context pack",
        "context projection source registry: my ticket context pack",
        "context projection section: my ticket context pack/usage_boundary",
        "context projection section: my ticket context pack/match_facts",
        "context projection section: my ticket context pack/user_order_slice",
        "context projection section: my ticket context pack/platform_order_summary",
        "context projection section: my ticket context pack/group_opinion_slice",
        "context projection section: my ticket context pack/retrieval_evidence",
        "context projection section: my ticket context pack/quality_feedback",
        "context projection source kind: my ticket context pack/user_order",
        "context projection source kind: my ticket context pack/ticket",
        "context projection source kind: my ticket context pack/context_audit",
        "scenario: group opinions has summary data",
        "quality unmatched cited sources",
        "quality non-synthetic adoption count"
    )
    if (-not $MissingPlatform) {
        $checks += "scenario: platform order has summary data"
    }
    $lines = @($checks | ForEach-Object { "OK`t$_" })
    Set-Content -LiteralPath $Path -Value $lines -Encoding UTF8
}

function Invoke-Fb2ProjectionValidationSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-context-projection-validation-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $failed = 0
        $goodLog = Join-Path $tempRoot "good-ai-center.log"
        New-Fb2ProjectionValidationSelfTestLog -Path $goodLog
        $goodStatus = [pscustomobject]@{
            latest_ai_center_context_projection = [ordered]@{
                path = $goodLog
                complete = $true
            }
        }
        $good = New-Fb2ProjectionValidationResult -Status $goodStatus -StatusPath "selftest-status.json" -LogPath ""
        if (-not [bool]$good.success) {
            $good | ConvertTo-Json -Depth 8
            $failed++
        }

        $missingFile = New-Fb2ProjectionValidationResult -Status $null -StatusPath "" -LogPath (Join-Path $tempRoot "missing.log")
        if ([bool]$missingFile.success) {
            $failed++
        }

        $badLog = Join-Path $tempRoot "bad-ai-center.log"
        New-Fb2ProjectionValidationSelfTestLog -Path $badLog -MissingPlatform
        $bad = New-Fb2ProjectionValidationResult -Status $null -StatusPath "" -LogPath $badLog
        if ([bool]$bad.success) {
            $failed++
        }

        $badStatus = [pscustomobject]@{
            latest_ai_center_context_projection = [ordered]@{
                path = $goodLog
                complete = $false
            }
        }
        $badStatusResult = New-Fb2ProjectionValidationResult -Status $badStatus -StatusPath "selftest-status-bad.json" -LogPath ""
        if ([bool]$badStatusResult.success) {
            $failed++
        }

        Write-Output "== SelfTest Summary =="
        Write-Output "failed=$failed"
        if ($failed -gt 0) {
            exit 1
        }
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2ProjectionValidationSelfTest
    exit 0
}

$root = Get-Fb2ProjectionValidationRepoRoot
if ([string]::IsNullOrWhiteSpace($StatusPath)) {
    $StatusPath = Join-Path $root "target\fb2-ai-center\status-current.json"
} else {
    $StatusPath = Resolve-Fb2ProjectionValidationPath -Path $StatusPath -Root $root
}
if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = Resolve-Fb2ProjectionValidationPath -Path $LogPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\context-projection-log-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2ProjectionValidationPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$status = Read-Fb2ProjectionValidationJson -Path $StatusPath
if ($null -eq $status -and [string]::IsNullOrWhiteSpace($LogPath)) {
    throw "Status or LogPath is required. Status not found: $StatusPath"
}
$result = New-Fb2ProjectionValidationResult -Status $status -StatusPath $StatusPath -LogPath $LogPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}

#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ExpectedHardwareSerial = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

function Get-SourceBlock {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Start,
        [Parameter(Mandatory = $true)][string]$End
    )

    $startIndex = $Source.IndexOf($Start, [StringComparison]::Ordinal)
    if ($startIndex -lt 0) { throw "Acceptance catalog start marker is missing: $Start" }
    $startIndex += $Start.Length
    $endIndex = $Source.IndexOf($End, $startIndex, [StringComparison]::Ordinal)
    if ($endIndex -lt 0) { throw "Acceptance catalog end marker is missing: $End" }
    return $Source.Substring($startIndex, $endIndex - $startIndex)
}

function Get-AcceptanceCatalog {
    $path = Join-Path $repoRoot `
        "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAcceptanceCaseCatalog.kt"
    $source = Get-Content -LiteralPath $path -Raw
    $manualBlock = Get-SourceBlock $source `
        'private val manualOnlyCases = setOf(' 'private val verificationCases = mapOf('
    $verificationBlock = Get-SourceBlock $source `
        'private val verificationCases = mapOf(' 'private val discoveryCases = mapOf('
    $discoveryBlock = Get-SourceBlock $source `
        'private val discoveryCases = mapOf(' 'fun verificationCase('
    $manual = @(
        [regex]::Matches($manualBlock, '"([a-z0-9_/-]+)"') |
            ForEach-Object { $_.Groups[1].Value } |
            Sort-Object -Unique
    )
    $verificationPairs = @(
        [regex]::Matches($verificationBlock, '"([a-z0-9_/-]+)"\s+to\s+"([a-z0-9_/-]+)"') |
            ForEach-Object {
                [pscustomobject]@{
                    feature_id = $_.Groups[1].Value
                    case_id = $_.Groups[2].Value
                }
            }
    )
    $discoveryPairs = @(
        [regex]::Matches($discoveryBlock, '"([a-z0-9_/-]+)"\s+to\s+"([a-z0-9_/-]+)"') |
            ForEach-Object {
                [pscustomobject]@{
                    feature_id = $_.Groups[1].Value
                    case_id = $_.Groups[2].Value
                }
            }
    )
    $allCases = @(
        @($verificationPairs.case_id) + @($discoveryPairs.case_id) |
            Sort-Object -Unique
    )
    return [pscustomobject]@{
        manual = $manual
        verification_pairs = $verificationPairs
        discovery_pairs = $discoveryPairs
        all_cases = $allCases
        scripted = @($allCases | Where-Object { $_ -notin $manual })
    }
}

function Get-GitValue {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    $value = & git -C $repoRoot @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) { return "unknown" }
    return ([string]($value | Select-Object -Last 1)).Trim()
}

function Write-AuditAtomically {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $directory = Split-Path -Parent $fullPath
    [System.IO.Directory]::CreateDirectory($directory) | Out-Null
    $temporary = "$fullPath.$PID.tmp"
    [System.IO.File]::WriteAllText(
        $temporary,
        ($Value | ConvertTo-Json -Depth 10),
        [System.Text.UTF8Encoding]::new($false)
    )
    Move-Item -LiteralPath $temporary -Destination $fullPath -Force
    return $fullPath
}

$catalog = Get-AcceptanceCatalog
if ($catalog.all_cases.Count -eq 0 -or $catalog.verification_pairs.Count -eq 0) {
    throw "Acceptance catalog did not expose any cases."
}
if (@($catalog.manual | Where-Object { $_ -notin $catalog.all_cases }).Count -gt 0) {
    throw "A manual-only acceptance case is not part of the feature catalog."
}
if (@($catalog.all_cases | Where-Object { $_ -notmatch '^[a-z0-9_/-]+$' }).Count -gt 0) {
    throw "Acceptance catalog contains an unsafe case id."
}

if ($SelfTest) {
    if ($catalog.manual.Count -ne 2 -or $catalog.scripted.Count -lt 1) {
        throw "Acceptance catalog self-test expected two manual-only cases and scripted coverage."
    }
    Write-Output "CHATGPT_WEB_GOAL_AUDIT_SELF_TEST=passed cases=$($catalog.all_cases.Count) scripted=$($catalog.scripted.Count) manual=$($catalog.manual.Count)"
    exit 0
}

$live = [ordered]@{
    status = if ($DeviceSerial) { "checking" } else { "not_requested" }
    app_version_name = $null
    app_version_code = $null
    adapter_version = $null
    ready_for_chat = $false
    ready_for_mcp = $false
    blocking_gap_count = $null
    unknown_capability_count = $null
    unknown_semantic_count = $null
    adaptation_review_required = $null
    current_case_count = $null
    current_case_ids = $null
    missing_scripted_case_ids = $null
    missing_manual_case_ids = $null
    implementation_remaining = $null
    code_remaining = $null
    verification_remaining = $null
    pending_verification_feature_ids = $null
    private_content_emitted = $false
}

if ($DeviceSerial) {
    . (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
    try {
        $runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
            -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
        Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            [string]$state.active_surface -ne "social_ai" -or
            [string]$state.social_chat.interaction_mode -ne "chat" -or
            [string]$state.social_chat.web_chat_provider_id -ne "chatgpt_web"
        ) {
            $live.status = "requires_chatgpt_production_surface"
        } else {
            $matrix = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                -Action "chatgpt_get_capability_matrix"
            $baseline = $matrix.feature_baseline
            $evidence = $baseline.verification_evidence
            $currentCases = @($evidence.current_case_ids | Sort-Object -Unique)
            $live.status = "ready"
            $live.app_version_name = [string]$matrix.app.version_name
            $live.app_version_code = [int]$matrix.app.version_code
            $live.adapter_version = [int]$matrix.adapter_version
            $live.ready_for_chat = $matrix.ready_for_chat -eq $true
            $live.ready_for_mcp = $matrix.ready_for_mcp -eq $true
            $live.blocking_gap_count = @($matrix.blocking_gaps).Count
            $live.unknown_capability_count = @($matrix.unknown_capabilities).Count
            $live.unknown_semantic_count = @($matrix.unknown_semantics).Count
            $live.adaptation_review_required = $matrix.adaptation_review.required -eq $true
            $live.current_case_count = $currentCases.Count
            $live.current_case_ids = $currentCases
            $live.missing_scripted_case_ids = @(
                $catalog.scripted | Where-Object { $_ -notin $currentCases }
            )
            $live.missing_manual_case_ids = @(
                $catalog.manual | Where-Object { $_ -notin $currentCases }
            )
            $live.implementation_remaining = [int]$baseline.summary.remaining
            $live.code_remaining = [int]$baseline.code_summary.remaining
            $live.verification_remaining = [int]$baseline.verification_summary.remaining
            $live.pending_verification_feature_ids = @(
                $baseline.pending_verification_feature_ids
            )
        }
    } catch {
        $live.status = "mcp_unavailable"
    }
}

$complete =
    $live.status -eq "ready" -and
    $live.ready_for_chat -and
    $live.ready_for_mcp -and
    $live.blocking_gap_count -eq 0 -and
    $live.unknown_capability_count -eq 0 -and
    $live.unknown_semantic_count -eq 0 -and
    -not $live.adaptation_review_required -and
    @($live.missing_scripted_case_ids).Count -eq 0 -and
    @($live.missing_manual_case_ids).Count -eq 0 -and
    $live.implementation_remaining -eq 0 -and
    $live.code_remaining -eq 0 -and
    $live.verification_remaining -eq 0

$audit = [ordered]@{
    schema = "elon.chatgpt_web.goal_audit.v1"
    generated_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
    source = [ordered]@{
        commit = Get-GitValue @("rev-parse", "HEAD")
        branch = Get-GitValue @("branch", "--show-current")
        verification_feature_count = $catalog.verification_pairs.Count
        discovery_feature_count = $catalog.discovery_pairs.Count
        unique_case_count = $catalog.all_cases.Count
        scripted_case_count = $catalog.scripted.Count
        manual_only_case_count = $catalog.manual.Count
        manual_only_case_ids = $catalog.manual
    }
    live = $live
    completion = [ordered]@{
        complete = $complete
        requires_production_surface = $live.status -eq "requires_chatgpt_production_surface"
        requires_user_supervision = if ($live.status -eq "ready") {
            @($live.missing_manual_case_ids).Count -gt 0
        } else {
            $true
        }
    }
}

if (-not $OutputPath) {
    $OutputPath = Join-Path $repoRoot "target/chatgpt-web/goal-audit-current.json"
}
$written = Write-AuditAtomically -Path $OutputPath -Value $audit
$audit | ConvertTo-Json -Depth 10
Write-Output "CHATGPT_WEB_GOAL_AUDIT_STATUS=$($live.status) complete=$complete"
Write-Output "CHATGPT_WEB_GOAL_AUDIT_PATH=$written"

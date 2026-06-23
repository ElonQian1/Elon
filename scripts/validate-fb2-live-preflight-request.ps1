#requires -Version 7.0

param(
    [string]$StatusPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2LivePreflightValidationRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2LivePreflightValidationPath {
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

function Get-Fb2LivePreflightValidationProperty {
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

function Read-Fb2LivePreflightValidationJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Status summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2LivePreflightValidationCheck {
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

function Test-Fb2LivePreflightValidationSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s`]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s`]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function Test-Fb2LivePreflightValidationContains {
    param(
        [object[]]$Items,
        [string]$Expected
    )

    return (@($Items | ForEach-Object { [string]$_ }) -contains $Expected)
}

function Get-Fb2LivePreflightValidationStatusObject {
    param(
        [object]$Source,
        [string]$Root
    )

    $live = Get-Fb2LivePreflightValidationProperty $Source "live_preflight_request"
    if ($null -ne $live) {
        return $Source
    }

    $files = Get-Fb2LivePreflightValidationProperty $Source "files"
    $statusPath = [string](Get-Fb2LivePreflightValidationProperty $files "status" "")
    if (-not [string]::IsNullOrWhiteSpace($statusPath)) {
        $statusPath = Resolve-Fb2LivePreflightValidationPath -Path $statusPath -Root $Root
        if (Test-Path -LiteralPath $statusPath) {
            return (Read-Fb2LivePreflightValidationJson -Path $statusPath)
        }
    }

    return $Source
}

function New-Fb2LivePreflightRequestValidation {
    param(
        [object]$Status,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $request = Get-Fb2LivePreflightValidationProperty $Status "live_preflight_request"
    Add-Fb2LivePreflightValidationCheck $checks "live preflight request present" ($null -ne $request)

    $schema = [string](Get-Fb2LivePreflightValidationProperty $request "schema" "")
    $missing = @(Get-Fb2LivePreflightValidationProperty $request "missing" @()) | ForEach-Object { [string]$_ }
    $nonTokenMissing = @($missing | Where-Object { $_ -ne "FB2_AI_CENTER_TOKEN" })
    $evidence = Get-Fb2LivePreflightValidationProperty $request "evidence_policy"
    $targetUser = Get-Fb2LivePreflightValidationProperty $request "target_user"
    $targetGroup = Get-Fb2LivePreflightValidationProperty $request "target_group"
    $commands = Get-Fb2LivePreflightValidationProperty $request "commands"
    $requiredFields = @(Get-Fb2LivePreflightValidationProperty $evidence "required_group_message_fields" @()) | ForEach-Object { [string]$_ }
    $gates = @(Get-Fb2LivePreflightValidationProperty $request "acceptance_gates" @()) | ForEach-Object { [string]$_ }

    Add-Fb2LivePreflightValidationCheck $checks "request schema" ($schema -eq "fb2.main_project.live_preflight_request.v1") "schema=$schema"
    Add-Fb2LivePreflightValidationCheck $checks "ready without token" ([bool](Get-Fb2LivePreflightValidationProperty $request "ready_without_token" $false))
    Add-Fb2LivePreflightValidationCheck $checks "token absent" (-not [bool](Get-Fb2LivePreflightValidationProperty $request "token_present" $true))
    Add-Fb2LivePreflightValidationCheck $checks "blocked by external secret" ([bool](Get-Fb2LivePreflightValidationProperty $request "blocked_by_external_secret" $false))
    Add-Fb2LivePreflightValidationCheck $checks "missing token only" (
        (Test-Fb2LivePreflightValidationContains -Items $missing -Expected "FB2_AI_CENTER_TOKEN") -and
        (@($nonTokenMissing).Count -eq 0)
    ) ("missing=$($missing -join ',')")
    Add-Fb2LivePreflightValidationCheck $checks "no write mode" ([bool](Get-Fb2LivePreflightValidationProperty $request "no_write_mode" $false))
    Add-Fb2LivePreflightValidationCheck $checks "does not write visible group messages" (-not [bool](Get-Fb2LivePreflightValidationProperty $request "writes_visible_group_messages" $true))

    Add-Fb2LivePreflightValidationCheck $checks "evidence uses direct api read" ([string](Get-Fb2LivePreflightValidationProperty $evidence "group_chat_test_method" "") -eq "direct_api_read")
    Add-Fb2LivePreflightValidationCheck $checks "evidence rejects screenshots" (-not [bool](Get-Fb2LivePreflightValidationProperty $evidence "screenshots_accepted" $true))
    Add-Fb2LivePreflightValidationCheck $checks "evidence read only summary schema" ([string](Get-Fb2LivePreflightValidationProperty $evidence "read_only_summary_schema" "") -eq "fb2.main_project.visible_chat_readonly.v1")
    foreach ($field in @("message_id", "text_len", "text_sha256")) {
        Add-Fb2LivePreflightValidationCheck $checks "required group message field $field" (Test-Fb2LivePreflightValidationContains -Items $requiredFields -Expected $field)
    }

    Add-Fb2LivePreflightValidationCheck $checks "target username" ([string](Get-Fb2LivePreflightValidationProperty $targetUser "fb2_username" "") -eq "123qwe")
    Add-Fb2LivePreflightValidationCheck $checks "target password placeholder" ([string](Get-Fb2LivePreflightValidationProperty $targetUser "fb2_password_placeholder" "") -eq "<FB2_PASSWORD>")
    Add-Fb2LivePreflightValidationCheck $checks "target external user id" ([string](Get-Fb2LivePreflightValidationProperty $targetUser "external_user_id" "") -eq "6fe5aa17-0403-427a-8e91-7f414beca35d")
    Add-Fb2LivePreflightValidationCheck $checks "target has historical order context" ([bool](Get-Fb2LivePreflightValidationProperty $targetUser "has_historical_order_context" $false))

    $sampleMessageId = [string](Get-Fb2LivePreflightValidationProperty $targetGroup "direct_read_sample_message_id" "")
    $sampleTextSha = [string](Get-Fb2LivePreflightValidationProperty $targetGroup "direct_read_sample_text_sha256" "")
    Add-Fb2LivePreflightValidationCheck $checks "target requested group" ([string](Get-Fb2LivePreflightValidationProperty $targetGroup "requested_group_id" "") -eq "official")
    Add-Fb2LivePreflightValidationCheck $checks "target resolved group" ([string](Get-Fb2LivePreflightValidationProperty $targetGroup "resolved_group_id" "") -eq "ext_fb2_official")
    Add-Fb2LivePreflightValidationCheck $checks "target sample message id present" (-not [string]::IsNullOrWhiteSpace($sampleMessageId))
    Add-Fb2LivePreflightValidationCheck $checks "target sample text sha256 present" ($sampleTextSha -match '^[a-fA-F0-9]{64}$')

    $noWriteCommand = [string](Get-Fb2LivePreflightValidationProperty $commands "no_write_direct_read" "")
    $dataCommand = [string](Get-Fb2LivePreflightValidationProperty $commands "data_only_preflight" "")
    $visibleCommand = [string](Get-Fb2LivePreflightValidationProperty $commands "visible_regression_requires_authorization" "")
    foreach ($pair in @(
            [ordered]@{ name = "no_write_direct_read"; value = $noWriteCommand },
            [ordered]@{ name = "data_only_preflight"; value = $dataCommand },
            [ordered]@{ name = "visible_regression_requires_authorization"; value = $visibleCommand }
        )) {
        Add-Fb2LivePreflightValidationCheck $checks "command $($pair.name) exists" (-not [string]::IsNullOrWhiteSpace([string]$pair.value))
        Add-Fb2LivePreflightValidationCheck $checks "command $($pair.name) secret safe" (Test-Fb2LivePreflightValidationSecretSafe -Text ([string]$pair.value))
    }

    Add-Fb2LivePreflightValidationCheck $checks "no write command is direct read only" (
        $noWriteCommand -match "ReadOnlyDirectRead" -and
        $noWriteCommand -notmatch "AllowVisibleMessages" -and
        $noWriteCommand -notmatch "Fb2AiCenterToken|FB2_AI_CENTER_TOKEN" -and
        $noWriteCommand -match "<FB2_PASSWORD>"
    )
    Add-Fb2LivePreflightValidationCheck $checks "data preflight command is no visible write" (
        $dataCommand -match "DataOnlyAcceptance" -and
        $dataCommand -match "PreflightOnly" -and
        $dataCommand -match "<FB2_AI_CENTER_TOKEN>" -and
        $dataCommand -match "<FB2_PASSWORD>" -and
        $dataCommand -notmatch "AllowVisibleMessages"
    )
    Add-Fb2LivePreflightValidationCheck $checks "visible regression command requires authorization" (
        $visibleCommand -match "DataOnlyAcceptance" -and
        $visibleCommand -match "AllowVisibleMessages" -and
        $visibleCommand -match "<FB2_AI_CENTER_TOKEN>" -and
        $visibleCommand -match "<FB2_PASSWORD>"
    )

    foreach ($gate in @(
            "fb2_authenticated_readiness_ready_or_partial_for_data_only",
            "context_pack_projection_complete",
            "permission_boundary_403_and_audit_summary",
            "quality_unmatched_cited_sources_zero",
            "feedback_coverage_complete",
            "direct_group_chat_read_text_hash_present",
            "user_scenario_audit_complete"
        )) {
        Add-Fb2LivePreflightValidationCheck $checks "acceptance gate $gate" (Test-Fb2LivePreflightValidationContains -Items $gates -Expected $gate)
    }

    $note = [string](Get-Fb2LivePreflightValidationProperty $request "note" "")
    Add-Fb2LivePreflightValidationCheck $checks "note present" (-not [string]::IsNullOrWhiteSpace($note))
    Add-Fb2LivePreflightValidationCheck $checks "note secret safe" (Test-Fb2LivePreflightValidationSecretSafe -Text $note)

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.live_preflight_request_validation.v1"
        source_status = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        ready_without_token = [bool](Get-Fb2LivePreflightValidationProperty $request "ready_without_token" $false)
        blocked_by_external_secret = [bool](Get-Fb2LivePreflightValidationProperty $request "blocked_by_external_secret" $false)
        token_present = [bool](Get-Fb2LivePreflightValidationProperty $request "token_present" $false)
        next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
    }
}

function New-Fb2LivePreflightSelfTestFixture {
    param(
        [switch]$ScreenshotAccepted,
        [switch]$VisibleWriteInDataPreflight,
        [switch]$RealSecret,
        [switch]$MissingHash
    )

    $password = if ($RealSecret) { "real-password-123456" } else { "<FB2_PASSWORD>" }
    $token = if ($RealSecret) { "real-token-1234567890" } else { "<FB2_AI_CENTER_TOKEN>" }
    $dataCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password $password -Fb2AiCenterToken $token"
    if ($VisibleWriteInDataPreflight) {
        $dataCommand = $dataCommand + " -AllowVisibleMessages"
    }

    [pscustomobject]@{
        live_preflight_request = [ordered]@{
            schema = "fb2.main_project.live_preflight_request.v1"
            ready_without_token = $true
            token_present = $false
            blocked_by_external_secret = $true
            missing = @("FB2_AI_CENTER_TOKEN")
            no_write_mode = $true
            writes_visible_group_messages = $false
            evidence_policy = [ordered]@{
                group_chat_test_method = "direct_api_read"
                screenshots_accepted = [bool]$ScreenshotAccepted
                required_group_message_fields = @("message_id", "text_len", "text_sha256")
                read_only_summary_schema = "fb2.main_project.visible_chat_readonly.v1"
            }
            target_user = [ordered]@{
                fb2_username = "123qwe"
                fb2_password_placeholder = "<FB2_PASSWORD>"
                external_user_id = "6fe5aa17-0403-427a-8e91-7f414beca35d"
                has_historical_order_context = $true
            }
            target_group = [ordered]@{
                requested_group_id = "official"
                resolved_group_id = "ext_fb2_official"
                direct_read_sample_message_id = "gai_sample"
                direct_read_sample_text_sha256 = if ($MissingHash) { "" } else { "b6f9bceebb28841a1380c002b3103e3d4264c8f1b4577a0af2855f537061fc1a" }
            }
            commands = [ordered]@{
                no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password $password"
                data_only_preflight = $dataCommand
                visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password $password -Fb2AiCenterToken $token"
            }
            acceptance_gates = @(
                "fb2_authenticated_readiness_ready_or_partial_for_data_only",
                "context_pack_projection_complete",
                "permission_boundary_403_and_audit_summary",
                "quality_unmatched_cited_sources_zero",
                "feedback_coverage_complete",
                "direct_group_chat_read_text_hash_present",
                "user_scenario_audit_complete"
            )
            note = "no_secret_handoff_for_refreshing_live_context_permission_quality_feedback_after_token_is_available"
        }
    }
}

function Invoke-Fb2LivePreflightValidationSelfTest {
    $failed = 0
    $good = New-Fb2LivePreflightSelfTestFixture
    $goodResult = New-Fb2LivePreflightRequestValidation -Status $good -SourcePath "selftest-good.json"
    if (-not [bool]$goodResult.success) {
        $goodResult | ConvertTo-Json -Depth 8
        $failed++
    }

    foreach ($case in @(
            [ordered]@{ name = "screenshots accepted"; fixture = (New-Fb2LivePreflightSelfTestFixture -ScreenshotAccepted) },
            [ordered]@{ name = "visible write in data preflight"; fixture = (New-Fb2LivePreflightSelfTestFixture -VisibleWriteInDataPreflight) },
            [ordered]@{ name = "real secret"; fixture = (New-Fb2LivePreflightSelfTestFixture -RealSecret) },
            [ordered]@{ name = "missing hash"; fixture = (New-Fb2LivePreflightSelfTestFixture -MissingHash) }
        )) {
        $result = New-Fb2LivePreflightRequestValidation -Status $case.fixture -SourcePath ("selftest-bad-" + $case.name + ".json")
        if ([bool]$result.success) {
            $failed++
        }
    }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2LivePreflightValidationSelfTest
    exit 0
}

$root = Get-Fb2LivePreflightValidationRepoRoot
if ([string]::IsNullOrWhiteSpace($StatusPath)) {
    $StatusPath = Join-Path $root "target\fb2-ai-center\status-current.json"
} else {
    $StatusPath = Resolve-Fb2LivePreflightValidationPath -Path $StatusPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\live-preflight-request-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2LivePreflightValidationPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$source = Read-Fb2LivePreflightValidationJson -Path $StatusPath
$status = Get-Fb2LivePreflightValidationStatusObject -Source $source -Root $root
$result = New-Fb2LivePreflightRequestValidation -Status $status -SourcePath $StatusPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}

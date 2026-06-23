#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [double]$MaxGeneratedAgeMinutes = 120,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2FreshnessRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2FreshnessPath {
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

function Get-Fb2FreshnessProperty {
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

function Read-Fb2FreshnessJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2FreshnessCheck {
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

function Test-Fb2FreshnessSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function ConvertTo-Fb2FreshnessDate {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }

    if ($Value -is [DateTimeOffset]) {
        return $Value.ToUniversalTime()
    }

    if ($Value -is [DateTime]) {
        $dateTime = [DateTime]$Value
        if ($dateTime.Kind -eq [DateTimeKind]::Unspecified) {
            $dateTime = [DateTime]::SpecifyKind($dateTime, [DateTimeKind]::Utc)
        }
        return ([DateTimeOffset]$dateTime).ToUniversalTime()
    }

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $null
    }

    try {
        $styles = [System.Globalization.DateTimeStyles]::AssumeUniversal -bor [System.Globalization.DateTimeStyles]::AdjustToUniversal
        return [DateTimeOffset]::Parse($text, [System.Globalization.CultureInfo]::InvariantCulture, $styles).ToUniversalTime()
    } catch {
        return $null
    }
}

function Find-Fb2FreshnessArtifact {
    param(
        [object[]]$Artifacts,
        [string]$Name
    )

    @($Artifacts | Where-Object { [string]$_.name -eq $Name } | Select-Object -First 1)
}

function New-Fb2FreshnessValidation {
    param(
        [object]$Refresh,
        [string]$SourcePath,
        [double]$MaxAgeMinutes
    )

    $checks = [System.Collections.ArrayList]::new()
    $freshness = Get-Fb2FreshnessProperty $Refresh "evidence_freshness"
    $files = Get-Fb2FreshnessProperty $Refresh "files"
    $artifacts = @(Get-Fb2FreshnessProperty $freshness "artifacts" @())

    Add-Fb2FreshnessCheck $checks "freshness schema" ([string](Get-Fb2FreshnessProperty $freshness "schema" "") -eq "fb2.main_project.evidence_freshness.v1")
    Add-Fb2FreshnessCheck $checks "freshness note says protected live needs token" ([string](Get-Fb2FreshnessProperty $freshness "note" "") -match "protected live fb2 data still requires FB2_AI_CENTER_TOKEN")
    Add-Fb2FreshnessCheck $checks "artifact count matches" ([int](Get-Fb2FreshnessProperty $freshness "artifact_count" 0) -eq @($artifacts).Count) ("declared=$([int](Get-Fb2FreshnessProperty $freshness 'artifact_count' 0)) actual=$(@($artifacts).Count)")

    $generatedAt = ConvertTo-Fb2FreshnessDate -Value (Get-Fb2FreshnessProperty $freshness "generated_at_utc" $null)
    Add-Fb2FreshnessCheck $checks "generated_at_utc parseable" ($null -ne $generatedAt)
    if ($null -ne $generatedAt) {
        $ageMinutes = ([DateTimeOffset]::UtcNow - $generatedAt).TotalMinutes
        Add-Fb2FreshnessCheck $checks "generated_at_utc not stale" ($ageMinutes -le $MaxAgeMinutes) ("age_minutes={0:N2} max={1:N2}" -f $ageMinutes, $MaxAgeMinutes)
    }

    Add-Fb2FreshnessCheck $checks "token flag matches refresh" (
        [bool](Get-Fb2FreshnessProperty $freshness "token_present" $false) -eq [bool](Get-Fb2FreshnessProperty $Refresh "token_present" $false)
    )
    Add-Fb2FreshnessCheck $checks "data goal flag matches refresh" (
        [bool](Get-Fb2FreshnessProperty $freshness "data_goal_complete" $false) -eq [bool](Get-Fb2FreshnessProperty $Refresh "data_goal_complete" $false)
    )
    Add-Fb2FreshnessCheck $checks "full final flag matches refresh" (
        [bool](Get-Fb2FreshnessProperty $freshness "full_final_complete" $false) -eq [bool](Get-Fb2FreshnessProperty $Refresh "full_final_complete" $false)
    )

    $currentArtifacts = @($artifacts | Where-Object { [bool]$_.exists -and [string]$_.source_scope -eq "current_output_dir" })
    $historyArtifacts = @($artifacts | Where-Object { [bool]$_.exists -and [string]$_.source_scope -eq "history_evidence_dir" })
    Add-Fb2FreshnessCheck $checks "current artifact count matches" ([int](Get-Fb2FreshnessProperty $freshness "current_output_artifact_count" 0) -eq @($currentArtifacts).Count)
    Add-Fb2FreshnessCheck $checks "history artifact count matches" ([int](Get-Fb2FreshnessProperty $freshness "history_artifact_count" 0) -eq @($historyArtifacts).Count)

    foreach ($artifact in $artifacts) {
        $name = [string](Get-Fb2FreshnessProperty $artifact "name" "")
        $path = [string](Get-Fb2FreshnessProperty $artifact "path" "")
        $scope = [string](Get-Fb2FreshnessProperty $artifact "source_scope" "")
        Add-Fb2FreshnessCheck $checks "artifact $name has path" (-not [string]::IsNullOrWhiteSpace($path))
        Add-Fb2FreshnessCheck $checks "artifact $name path secret safe" (Test-Fb2FreshnessSecretSafe -Text $path)
        Add-Fb2FreshnessCheck $checks "artifact $name scope valid" (@("current_output_dir", "history_evidence_dir", "missing") -contains $scope)
    }

    foreach ($name in @(
            "public_contract_status",
            "status",
            "goal_audit",
            "goal_audit_markdown",
            "handoff",
            "handoff_markdown",
            "status_refresh",
            "handoff_prompt"
        )) {
        $artifact = Find-Fb2FreshnessArtifact -Artifacts $artifacts -Name $name
        Add-Fb2FreshnessCheck $checks "required artifact $name present" (@($artifact).Count -gt 0)
        if (@($artifact).Count -gt 0) {
            $artifactItem = @($artifact)[0]
            Add-Fb2FreshnessCheck $checks "required artifact $name exists" ([bool](Get-Fb2FreshnessProperty $artifactItem "exists" $false))
            Add-Fb2FreshnessCheck $checks "required artifact $name is current output" ([string](Get-Fb2FreshnessProperty $artifactItem "source_scope" "") -eq "current_output_dir")
        }
    }

    foreach ($name in @("status_refresh", "handoff_prompt")) {
        $artifact = Find-Fb2FreshnessArtifact -Artifacts $artifacts -Name $name
        if (@($artifact).Count -gt 0) {
            $artifactItem = @($artifact)[0]
            Add-Fb2FreshnessCheck $checks "generated artifact $name age zero in summary" ([double](Get-Fb2FreshnessProperty $artifactItem "age_minutes" -1) -eq 0.0)
        }
    }

    $exportedSamplePath = [string](Get-Fb2FreshnessProperty $files "exported_context_pack_sample_set_validation" "")
    if (-not [string]::IsNullOrWhiteSpace($exportedSamplePath)) {
        $artifact = Find-Fb2FreshnessArtifact -Artifacts $artifacts -Name "exported_context_pack_sample_set_validation"
        Add-Fb2FreshnessCheck $checks "exported sample freshness artifact present" (@($artifact).Count -gt 0)
        if (@($artifact).Count -gt 0) {
            $artifactItem = @($artifact)[0]
            Add-Fb2FreshnessCheck $checks "exported sample freshness artifact exists" ([bool](Get-Fb2FreshnessProperty $artifactItem "exists" $false))
            Add-Fb2FreshnessCheck $checks "exported sample freshness path matches files" (
                [string](Get-Fb2FreshnessProperty $artifactItem "path" "") -eq $exportedSamplePath
            )
        }
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.evidence_freshness_validation.v1"
        source_refresh = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }
}

function Invoke-Fb2FreshnessSelfTest {
    $now = [DateTimeOffset]::UtcNow
    $fixture = [pscustomobject]@{
        token_present = $false
        data_goal_complete = $true
        full_final_complete = $false
        files = [ordered]@{
            exported_context_pack_sample_set_validation = "target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
        }
        evidence_freshness = [ordered]@{
            schema = "fb2.main_project.evidence_freshness.v1"
            generated_at_utc = $now.ToString("o")
            note = "artifact freshness only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN"
            current_output_dir = "target\fb2-ai-center"
            evidence_dirs = @("target\fb2-ai-center")
            artifact_count = 9
            current_output_artifact_count = 9
            history_artifact_count = 0
            token_present = $false
            data_goal_complete = $true
            full_final_complete = $false
            artifacts = @(
                [ordered]@{ name = "public_contract_status"; path = "target\fb2-ai-center\public-contract-status-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 },
                [ordered]@{ name = "status"; path = "target\fb2-ai-center\status-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 },
                [ordered]@{ name = "goal_audit"; path = "target\fb2-ai-center\goal-audit-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 },
                [ordered]@{ name = "goal_audit_markdown"; path = "target\fb2-ai-center\goal-audit-current.md"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 },
                [ordered]@{ name = "handoff"; path = "target\fb2-ai-center\handoff-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 },
                [ordered]@{ name = "handoff_markdown"; path = "target\fb2-ai-center\handoff-current.md"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.0 },
                [ordered]@{ name = "status_refresh"; path = "target\fb2-ai-center\status-refresh-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.0 },
                [ordered]@{ name = "handoff_prompt"; path = "target\fb2-ai-center\handoff-prompt-current.md"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.0 },
                [ordered]@{ name = "exported_context_pack_sample_set_validation"; path = "target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"; exists = $true; source_scope = "current_output_dir"; last_write_utc = $now.ToString("o"); age_minutes = 0.01 }
            )
        }
    }

    $failed = 0
    $good = New-Fb2FreshnessValidation -Refresh $fixture -SourcePath "selftest-good.json" -MaxAgeMinutes 120
    if (-not [bool]$good.success) {
        $good | ConvertTo-Json -Depth 8
        $failed++
    }

    $stale = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $stale.evidence_freshness.generated_at_utc = $now.AddHours(-4).ToString("o")
    $staleResult = New-Fb2FreshnessValidation -Refresh $stale -SourcePath "selftest-stale.json" -MaxAgeMinutes 120
    if ([bool]$staleResult.success) { $failed++ }

    $missingArtifact = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $missingArtifact.evidence_freshness.artifacts = @($missingArtifact.evidence_freshness.artifacts | Where-Object { [string]$_.name -ne "handoff_prompt" })
    $missingArtifact.evidence_freshness.artifact_count = @($missingArtifact.evidence_freshness.artifacts).Count
    $missingArtifact.evidence_freshness.current_output_artifact_count = @($missingArtifact.evidence_freshness.artifacts).Count
    $missingResult = New-Fb2FreshnessValidation -Refresh $missingArtifact -SourcePath "selftest-missing-handoff-prompt.json" -MaxAgeMinutes 120
    if ([bool]$missingResult.success) { $failed++ }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2FreshnessSelfTest
    exit 0
}

$root = Get-Fb2FreshnessRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2FreshnessPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\evidence-freshness-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2FreshnessPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2FreshnessJson -Path $RefreshPath
$result = New-Fb2FreshnessValidation -Refresh $refresh -SourcePath $RefreshPath -MaxAgeMinutes $MaxGeneratedAgeMinutes
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}

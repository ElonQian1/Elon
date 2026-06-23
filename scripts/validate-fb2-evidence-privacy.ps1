#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$StatusPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

$script:Fb2PrivacyRawFieldNames = @(
    "text",
    "content",
    "body",
    "message_text",
    "raw_text",
    "sample_text",
    "full_text",
    "order_detail",
    "order_items",
    "ticket_detail",
    "user_identity",
    "phone",
    "mobile",
    "id_card"
)

function Get-Fb2PrivacyRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2PrivacyPath {
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

function Get-Fb2PrivacyProperty {
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

function Add-Fb2PrivacyCheck {
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

function Test-Fb2PrivacySecretSafe {
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
    if ($Text -match '(?i)"(?:FB2_AI_CENTER_TOKEN|FB2_MAIN_PROJECT_SHARED_SECRET|token|password|secret)"\s*:\s*"(?!<)[^"]{6,}"') {
        return $false
    }
    if ($Text -match '(?i)123qwe/123qwe') {
        return $false
    }
    return $true
}

function Read-Fb2PrivacyJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "JSON artifact not found: $Path"
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Find-Fb2PrivacyRawEvidence {
    param(
        [object]$Node,
        [string]$Path = "$"
    )

    $findings = [System.Collections.ArrayList]::new()
    if ($null -eq $Node) {
        return @($findings)
    }

    if ($Node -is [string]) {
        $value = [string]$Node
        if ($value -match '(?i)(^|[\s,;])(?:text|content|body|message_text|raw_text|sample_text|full_text)\s*=') {
            [void]$findings.Add([ordered]@{
                path = $Path
                reason = "raw_text_like_assignment"
            })
        }
        if ($value -match '(?i)<fb2_context_pack\b') {
            [void]$findings.Add([ordered]@{
                path = $Path
                reason = "raw_context_pack_body"
            })
        }
        return @($findings)
    }

    if ($Node -is [System.ValueType]) {
        return @($findings)
    }

    if ($Node -is [System.Collections.IDictionary]) {
        foreach ($key in @($Node.Keys)) {
            $name = [string]$key
            $lowerName = $name.ToLowerInvariant()
            $childPath = "$Path.$name"
            if ($script:Fb2PrivacyRawFieldNames -contains $lowerName) {
                [void]$findings.Add([ordered]@{
                    path = $childPath
                    reason = "forbidden_raw_field:$name"
                })
            }
            foreach ($finding in @(Find-Fb2PrivacyRawEvidence -Node $Node[$key] -Path $childPath)) {
                [void]$findings.Add($finding)
            }
        }
        return @($findings)
    }

    if ($Node -is [array] -or $Node -is [System.Collections.IList]) {
        $index = 0
        foreach ($item in @($Node)) {
            foreach ($finding in @(Find-Fb2PrivacyRawEvidence -Node $item -Path "${Path}[$index]")) {
                [void]$findings.Add($finding)
            }
            $index++
        }
        return @($findings)
    }

    $typeName = $Node.GetType().FullName
    if ($typeName -notin @("System.Management.Automation.PSCustomObject", "System.Dynamic.ExpandoObject")) {
        return @($findings)
    }

    foreach ($property in @($Node.PSObject.Properties)) {
        $name = [string]$property.Name
        $lowerName = $name.ToLowerInvariant()
        $childPath = "$Path.$name"
        if ($script:Fb2PrivacyRawFieldNames -contains $lowerName) {
            [void]$findings.Add([ordered]@{
                path = $childPath
                reason = "forbidden_raw_field:$name"
            })
        }
        foreach ($finding in @(Find-Fb2PrivacyRawEvidence -Node $property.Value -Path $childPath)) {
            [void]$findings.Add($finding)
        }
    }
    return @($findings)
}

function Test-Fb2PrivacyHash {
    param([string]$Value)

    $Value -match '^[a-fA-F0-9]{64}$'
}

function New-Fb2PrivacyValidation {
    param(
        [string]$RefreshPath,
        [string]$StatusPath,
        [string]$OutputPath
    )

    $root = Get-Fb2PrivacyRepoRoot
    if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
        $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
    } else {
        $RefreshPath = Resolve-Fb2PrivacyPath -Path $RefreshPath -Root $root
    }
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        $OutputPath = Join-Path $root "target\fb2-ai-center\evidence-privacy-validation-current.json"
    } else {
        $OutputPath = Resolve-Fb2PrivacyPath -Path $OutputPath -Root $root
    }

    $checks = [System.Collections.ArrayList]::new()
    $filesChecked = [System.Collections.ArrayList]::new()
    $rawFindings = [System.Collections.ArrayList]::new()
    $secretUnsafeFiles = [System.Collections.ArrayList]::new()

    $refresh = Read-Fb2PrivacyJson -Path $RefreshPath
    $files = Get-Fb2PrivacyProperty $refresh "files"
    if ([string]::IsNullOrWhiteSpace($StatusPath)) {
        $StatusPath = [string](Get-Fb2PrivacyProperty $files "status" "")
    }
    if ([string]::IsNullOrWhiteSpace($StatusPath)) {
        $StatusPath = Join-Path $root "target\fb2-ai-center\status-current.json"
    } else {
        $StatusPath = Resolve-Fb2PrivacyPath -Path $StatusPath -Root $root
    }

    $artifactPaths = [System.Collections.Generic.List[string]]::new()
    $candidatePaths = [System.Collections.ArrayList]::new()
    [void]$candidatePaths.Add($RefreshPath)
    [void]$candidatePaths.Add($StatusPath)
    if ($null -ne $files) {
        foreach ($property in @($files.PSObject.Properties)) {
            $candidate = Resolve-Fb2PrivacyPath -Path ([string]$property.Value) -Root $root
            if (($property.Name -in @("token_bridge_live_preflight", "token_bridge_live_preflight_summary")) -and -not (Test-Path -LiteralPath $candidate)) {
                continue
            }
            [void]$candidatePaths.Add($candidate)
        }
    }
    foreach ($path in @($candidatePaths)) {
        if (-not [string]::IsNullOrWhiteSpace($path)) {
            $resolved = Resolve-Fb2PrivacyPath -Path $path -Root $root
            if (-not $artifactPaths.Contains($resolved)) {
                $artifactPaths.Add($resolved)
            }
        }
    }

    $bridgeResultPath = Join-Path $root "target\fb2-ai-center\token-bridge-data-only-preflight-current.json"
    if (Test-Path -LiteralPath $bridgeResultPath) {
        if (-not $artifactPaths.Contains($bridgeResultPath)) {
            $artifactPaths.Add($bridgeResultPath)
        }
        try {
            $bridgeResult = Read-Fb2PrivacyJson -Path $bridgeResultPath
            $bridgeSummaryPath = [string](Get-Fb2PrivacyProperty $bridgeResult "summary_path" "")
            if (-not [string]::IsNullOrWhiteSpace($bridgeSummaryPath)) {
                $resolvedSummaryPath = Resolve-Fb2PrivacyPath -Path $bridgeSummaryPath -Root $root
                if (-not $artifactPaths.Contains($resolvedSummaryPath)) {
                    $artifactPaths.Add($resolvedSummaryPath)
                }
            }
        } catch {
            Add-Fb2PrivacyCheck $checks "token bridge result parseable for privacy expansion" $false $_.Exception.Message
        }
    }

    foreach ($path in @($artifactPaths)) {
        $exists = Test-Path -LiteralPath $path
        [void]$filesChecked.Add([ordered]@{
            path = $path
            exists = $exists
        })
        Add-Fb2PrivacyCheck $checks "artifact exists $([System.IO.Path]::GetFileName($path))" $exists $path
        if (-not $exists) {
            continue
        }

        $rawText = Get-Content -LiteralPath $path -Raw
        if (-not (Test-Fb2PrivacySecretSafe -Text $rawText)) {
            [void]$secretUnsafeFiles.Add($path)
        }

        $extension = [System.IO.Path]::GetExtension($path)
        $looksLikeJson = $extension -ieq ".json" -or $rawText.TrimStart().StartsWith("{") -or $rawText.TrimStart().StartsWith("[")
        if ($looksLikeJson) {
            try {
                $json = $rawText | ConvertFrom-Json
                foreach ($finding in @(Find-Fb2PrivacyRawEvidence -Node $json -Path ([System.IO.Path]::GetFileName($path)))) {
                    [void]$rawFindings.Add($finding)
                }
            } catch {
                Add-Fb2PrivacyCheck $checks "artifact parseable $([System.IO.Path]::GetFileName($path))" $false $_.Exception.Message
            }
        }
    }

    Add-Fb2PrivacyCheck $checks "no secret values in evidence artifacts" (@($secretUnsafeFiles).Count -eq 0) ((@($secretUnsafeFiles) -join "; "))
    Add-Fb2PrivacyCheck $checks "no raw text/order fields in evidence artifacts" (@($rawFindings).Count -eq 0) ((@($rawFindings | ForEach-Object { "$($_.path):$($_.reason)" }) -join "; "))

    $status = if (Test-Path -LiteralPath $StatusPath) { Read-Fb2PrivacyJson -Path $StatusPath } else { $null }
    $latestReadOnly = Get-Fb2PrivacyProperty $status "latest_read_only_direct_read"
    $readOnlyEvidence = Get-Fb2PrivacyProperty $latestReadOnly "evidence"
    $readOnlyComplete = [bool](Get-Fb2PrivacyProperty $latestReadOnly "complete" $false)
    if ($readOnlyComplete) {
        $sampleTextLen = [int](Get-Fb2PrivacyProperty $readOnlyEvidence "sample_text_len" 0)
        $sampleTextSha = [string](Get-Fb2PrivacyProperty $readOnlyEvidence "sample_text_sha256" "")
        Add-Fb2PrivacyCheck $checks "read-only group evidence uses text_len" ($sampleTextLen -gt 0) "sample_text_len=$sampleTextLen"
        Add-Fb2PrivacyCheck $checks "read-only group evidence uses text_sha256" (Test-Fb2PrivacyHash -Value $sampleTextSha) "sample_text_sha256=$sampleTextSha"
    }

    $scenarioAudit = Get-Fb2PrivacyProperty $status "latest_user_scenario_audit"
    $scenarios = @(Get-Fb2PrivacyProperty $scenarioAudit "scenarios" @())
    $hashBearingScenarioCount = @($scenarios | Where-Object {
            $scenarioText = $_ | ConvertTo-Json -Depth 12
            $scenarioText -match 'text_sha256=' -or $scenarioText -match '"context_pack_sha256"'
        }).Count
    Add-Fb2PrivacyCheck $checks "scenario audit evidence uses hashes" ($hashBearingScenarioCount -gt 0) "hash_bearing_scenarios=$hashBearingScenarioCount"

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    $result = [ordered]@{
        schema = "fb2.main_project.evidence_privacy_validation.v1"
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        source_refresh = $RefreshPath
        source_status = $StatusPath
        output_path = $OutputPath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        checked_file_count = @($filesChecked).Count
        checked_files = @($filesChecked)
        raw_findings = @($rawFindings)
        secret_unsafe_files = @($secretUnsafeFiles)
        note = "Validates local fb2 AI Center artifacts store source IDs, text_len and text_sha256 rather than message/order bodies."
    }

    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $result | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    $result
}

function Invoke-Fb2PrivacySelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-evidence-privacy-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $statusPath = Join-Path $tempRoot "status.json"
        $goalPath = Join-Path $tempRoot "goal.json"
        $handoffPath = Join-Path $tempRoot "handoff.json"
        $handoffMarkdownPath = Join-Path $tempRoot "handoff.md"
        $handoffPromptPath = Join-Path $tempRoot "handoff-prompt.md"
        $publicPath = Join-Path $tempRoot "public.json"
        $samplesPath = Join-Path $tempRoot "samples.json"
        $refreshPath = Join-Path $tempRoot "refresh.json"
        $outputPath = Join-Path $tempRoot "out.json"
        $hashA = "a" * 64
        $hashB = "b" * 64
        $hashC = "c" * 64
        $hashD = "d" * 64

        $status = [ordered]@{
            schema = "fb2.main_project.status_snapshot.v1"
            latest_read_only_direct_read = [ordered]@{
                complete = $true
                evidence = [ordered]@{
                    sample_message_id = "msg_1"
                    sample_text_len = 24
                    sample_text_sha256 = $hashA
                }
            }
            latest_user_scenario_audit = [ordered]@{
                scenarios = @(
                    [ordered]@{
                        id = "selected_message_review"
                        evidence = "message=msg_1 text_len=24 text_sha256=$hashB"
                    }
                )
            }
        }
        $refresh = [ordered]@{
            schema = "fb2.main_project.status_refresh.v1"
            files = [ordered]@{
                status = $statusPath
                goal_audit = $goalPath
                handoff = $handoffPath
                handoff_markdown = $handoffMarkdownPath
                handoff_prompt = $handoffPromptPath
                public_contract_status = $publicPath
                exported_context_pack_sample_set_validation = $samplesPath
            }
        }
        foreach ($pair in @(
                @($statusPath, $status),
                @($refreshPath, $refresh),
                @($goalPath, [ordered]@{ evidence = "text_len=12 text_sha256=$hashC" }),
                @($handoffPath, [ordered]@{ command = "-Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>" }),
                @($publicPath, [ordered]@{ schema = "ok" }),
                @($samplesPath, [ordered]@{ context_pack_sha256 = $hashD })
            )) {
            $pair[1] | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $pair[0] -Encoding UTF8
        }
        Set-Content -LiteralPath $handoffMarkdownPath -Value "safe handoff uses -Fb2Password <FB2_PASSWORD>" -Encoding UTF8
        Set-Content -LiteralPath $handoffPromptPath -Value "safe prompt uses <FB2_AI_CENTER_TOKEN>" -Encoding UTF8

        $good = New-Fb2PrivacyValidation -RefreshPath $refreshPath -OutputPath $outputPath
        $badSecretPath = Join-Path $tempRoot "bad-secret.json"
        $badSecretRefreshPath = Join-Path $tempRoot "bad-secret-refresh.json"
        [ordered]@{ command = "-Fb2AiCenterToken real-secret-token-1234567890" } | ConvertTo-Json | Set-Content -LiteralPath $badSecretPath -Encoding UTF8
        [ordered]@{ schema = "fb2.main_project.status_refresh.v1"; files = [ordered]@{ status = $badSecretPath } } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badSecretRefreshPath -Encoding UTF8
        $badSecret = New-Fb2PrivacyValidation -RefreshPath $badSecretRefreshPath -OutputPath (Join-Path $tempRoot "bad-secret-out.json")

        $badRawPath = Join-Path $tempRoot "bad-raw.json"
        $badRawRefreshPath = Join-Path $tempRoot "bad-raw-refresh.json"
        [ordered]@{ recent_messages = @([ordered]@{ message_id = "m1"; text = "raw body" }) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badRawPath -Encoding UTF8
        [ordered]@{ schema = "fb2.main_project.status_refresh.v1"; files = [ordered]@{ status = $badRawPath } } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badRawRefreshPath -Encoding UTF8
        $badRaw = New-Fb2PrivacyValidation -RefreshPath $badRawRefreshPath -OutputPath (Join-Path $tempRoot "bad-raw-out.json")

        $badEvidencePath = Join-Path $tempRoot "bad-evidence.json"
        $badEvidenceRefreshPath = Join-Path $tempRoot "bad-evidence-refresh.json"
        [ordered]@{ evidence = "message=m1 text=raw body" } | ConvertTo-Json | Set-Content -LiteralPath $badEvidencePath -Encoding UTF8
        [ordered]@{ schema = "fb2.main_project.status_refresh.v1"; files = [ordered]@{ status = $badEvidencePath } } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badEvidenceRefreshPath -Encoding UTF8
        $badEvidence = New-Fb2PrivacyValidation -RefreshPath $badEvidenceRefreshPath -OutputPath (Join-Path $tempRoot "bad-evidence-out.json")

        $badMarkdownPath = Join-Path $tempRoot "bad-secret.md"
        $badMarkdownRefreshPath = Join-Path $tempRoot "bad-markdown-refresh.json"
        Set-Content -LiteralPath $badMarkdownPath -Value "pwsh -File smoke.ps1 -Fb2Password 123qwe" -Encoding UTF8
        [ordered]@{ schema = "fb2.main_project.status_refresh.v1"; files = [ordered]@{ status = $statusPath; handoff_markdown = $badMarkdownPath } } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badMarkdownRefreshPath -Encoding UTF8
        $badMarkdown = New-Fb2PrivacyValidation -RefreshPath $badMarkdownRefreshPath -OutputPath (Join-Path $tempRoot "bad-markdown-out.json")

        $failed = 0
        if (-not [bool]$good.success) { $failed++ }
        if ([bool]$badSecret.success) { $failed++ }
        if ([bool]$badRaw.success) { $failed++ }
        if ([bool]$badEvidence.success) { $failed++ }
        if ([bool]$badMarkdown.success) { $failed++ }

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
    Invoke-Fb2PrivacySelfTest
    exit 0
}

$result = New-Fb2PrivacyValidation -RefreshPath $RefreshPath -StatusPath $StatusPath -OutputPath $OutputPath
$result | ConvertTo-Json -Depth 40
if (-not [bool]$result.success) {
    exit 1
}

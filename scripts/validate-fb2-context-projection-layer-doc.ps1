#requires -Version 7.0

param(
    [string]$DocumentPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2ProjectionLayerRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2ProjectionLayerPath {
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

function Add-Fb2ProjectionLayerCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = [bool]$Passed
        details = $Details
    })
}

function Test-Fb2ProjectionLayerSecretSafe {
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

function New-Fb2ProjectionLayerValidation {
    param(
        [string]$Path,
        [string]$OutputPath
    )

    $checks = [System.Collections.ArrayList]::new()
    $exists = -not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path -LiteralPath $Path)
    Add-Fb2ProjectionLayerCheck $checks "document exists" $exists $Path
    $content = if ($exists) { Get-Content -LiteralPath $Path -Raw } else { "" }

    Add-Fb2ProjectionLayerCheck $checks "schema present" ($content -match "fb2\.main_project\.context_projection_layer\.v1")
    Add-Fb2ProjectionLayerCheck $checks "xml wrapped markdown" ($content -match "<fb2_context_pack" -and $content -match "</fb2_context_pack>")
    Add-Fb2ProjectionLayerCheck $checks "rest first delivery" ($content -match "REST Context Pack" -and $content -match "tool manifest" -and $content -match "tools/execute")
    Add-Fb2ProjectionLayerCheck $checks "mcp is future wrapper" ($content -match "MCP" -and $content -match "wrapper" -and $content -match "must not replace")
    Add-Fb2ProjectionLayerCheck $checks "direct read evidence policy" ($content -match "direct API read" -and $content -match "text_len" -and $content -match "text_sha256")

    foreach ($lane in @(
            "match_facts_and_odds",
            "current_user_tickets",
            "platform_order_summary",
            "group_opinions",
            "opinion_learning_loop",
            "quality_feedback_audit"
        )) {
        Add-Fb2ProjectionLayerCheck $checks "lane $lane" ($content -match [regex]::Escape($lane))
    }

    foreach ($index in @(
            "match_index",
            "odds_snapshot_index",
            "current_user_ticket_index",
            "platform_order_risk_index",
            "group_opinion_index",
            "opinion_memory_index",
            "context_audit_index",
            "feedback_quality_index"
        )) {
        Add-Fb2ProjectionLayerCheck $checks "index $index" ($content -match [regex]::Escape($index))
    }

    foreach ($scenario in @(
            "today_matches_analysis",
            "my_ticket_analysis",
            "platform_order_risk",
            "group_opinion_summary",
            "selected_message_review",
            "group_discussion_summary_post",
            "source_reference_audit"
        )) {
        Add-Fb2ProjectionLayerCheck $checks "scenario $scenario" ($content -match [regex]::Escape($scenario))
    }

    foreach ($forbidden in @(
            "fabricated_odds",
            "guaranteed_win",
            "other_user_order_detail",
            "single_user_order_detail",
            "user_identity_leak",
            "fabricated_group_view",
            "group_opinion_as_fact",
            "uncited_source",
            "raw_embedding_dump",
            "full_database_dump"
        )) {
        Add-Fb2ProjectionLayerCheck $checks "forbidden output $forbidden" ($content -match [regex]::Escape($forbidden))
    }

    Add-Fb2ProjectionLayerCheck $checks "secret safe" (Test-Fb2ProjectionLayerSecretSafe -Text $content)

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    $result = [ordered]@{
        schema = "fb2.main_project.context_projection_layer_doc_validation.v1"
        generated_at_utc = ([datetime]::UtcNow).ToString("o")
        source_document = $Path
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }

    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $parent = Split-Path -Parent $OutputPath
        if (-not [string]::IsNullOrWhiteSpace($parent)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    }
    $result
}

function Invoke-Fb2ProjectionLayerSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-projection-layer-doc-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $goodPath = Join-Path $tempRoot "good.md"
        $badPath = Join-Path $tempRoot "bad.md"
        $secretPath = Join-Path $tempRoot "secret.md"

        $root = Get-Fb2ProjectionLayerRepoRoot
        $realDoc = Join-Path $root "docs\fb2-ai-center\context-projection-layer.md"
        if (Test-Path -LiteralPath $realDoc) {
            Copy-Item -LiteralPath $realDoc -Destination $goodPath
        } else {
            throw "SelfTest fixture source missing: $realDoc"
        }

        (Get-Content -LiteralPath $goodPath -Raw).Replace("match_facts_and_odds", "missing_lane") |
            Set-Content -LiteralPath $badPath -Encoding UTF8
        (Get-Content -LiteralPath $goodPath -Raw) + "`nFB2_AI_CENTER_TOKEN=secret-real-value-1234567890" |
            Set-Content -LiteralPath $secretPath -Encoding UTF8

        $good = New-Fb2ProjectionLayerValidation -Path $goodPath -OutputPath ""
        $bad = New-Fb2ProjectionLayerValidation -Path $badPath -OutputPath ""
        $secret = New-Fb2ProjectionLayerValidation -Path $secretPath -OutputPath ""
        $failed = 0
        if (-not [bool]$good.success) { $failed++ }
        if ([bool]$bad.success) { $failed++ }
        if ([bool]$secret.success) { $failed++ }

        "== SelfTest Summary =="
        "failed=$failed"
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
    Invoke-Fb2ProjectionLayerSelfTest
    exit 0
}

$root = Get-Fb2ProjectionLayerRepoRoot
if ([string]::IsNullOrWhiteSpace($DocumentPath)) {
    $DocumentPath = Join-Path $root "docs\fb2-ai-center\context-projection-layer.md"
} else {
    $DocumentPath = Resolve-Fb2ProjectionLayerPath -Path $DocumentPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\context-projection-layer-doc-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2ProjectionLayerPath -Path $OutputPath -Root $root
}

$validation = New-Fb2ProjectionLayerValidation -Path $DocumentPath -OutputPath $OutputPath
$json = $validation | ConvertTo-Json -Depth 8
$json

if (-not [bool]$validation.success) {
    exit 1
}

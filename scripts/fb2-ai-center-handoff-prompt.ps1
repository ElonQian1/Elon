#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2PromptRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2PromptPath {
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

function Get-Fb2PromptProperty {
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

function Read-Fb2PromptJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function ConvertTo-Fb2PromptText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Protect-Fb2PromptSecret {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return ""
    }

    $redacted = $Text -replace "(?i)(FB2_AI_CENTER_TOKEN\s*=\s*)['""][^'""]+['""]", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2AiCenterToken\s+)(?!<FB2_AI_CENTER_TOKEN>)[^\s]+", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2Token\s+)(?!<FB2_AI_CENTER_TOKEN>)[^\s]+", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2Password\s+)(?!<FB2_PASSWORD>)[^\s]+", '${1}<FB2_PASSWORD>'
    return $redacted
}

function Format-Fb2PromptCell {
    param(
        [object]$Value,
        [int]$MaxLength = 180
    )

    $text = Protect-Fb2PromptSecret -Text (ConvertTo-Fb2PromptText $Value)
    $text = $text -replace "(`r`n|`n|`r)", " "
    $text = $text.Replace("|", "/")
    if ($text.Length -gt $MaxLength) {
        return ($text.Substring(0, $MaxLength) + "...")
    }
    return $text
}

function Add-Fb2PromptLine {
    param(
        [System.Collections.ArrayList]$Lines,
        [string]$Text = ""
    )

    [void]$Lines.Add($Text)
}

function New-Fb2HandoffPrompt {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $matrix = Get-Fb2PromptProperty $Refresh "completion_matrix"
    $gates = Get-Fb2PromptProperty $matrix "gates"
    $totals = Get-Fb2PromptProperty $matrix "totals"
    $commands = Get-Fb2PromptProperty $Refresh "next_commands"
    $blocking = Get-Fb2PromptProperty $Refresh "blocking_state"
    $ownerActions = Get-Fb2PromptProperty $Refresh "owner_next_actions"
    $freshness = Get-Fb2PromptProperty $Refresh "evidence_freshness"
    $freshnessArtifacts = @(Get-Fb2PromptProperty $freshness "artifacts" @())
    $gapBoard = Get-Fb2PromptProperty $Refresh "gap_action_board"
    $gapActions = @(Get-Fb2PromptProperty $gapBoard "actions" @())
    $requirements = @(Get-Fb2PromptProperty $matrix "requirements" @())
    $lines = [System.Collections.ArrayList]::new()

    Add-Fb2PromptLine -Lines $lines -Text "# fb2 AI Center 下一轮执行提示"
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text ('来源 refresh summary: `{0}`' -f $SourcePath)
    Add-Fb2PromptLine -Lines $lines -Text ('schema: `{0}` / matrix: `{1}`' -f [string]$Refresh.schema, [string]$matrix.schema)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 当前闸门"
    Add-Fb2PromptLine -Lines $lines -Text ('- data_goal_complete: `{0}`' -f [bool]$gates.data_goal_complete)
    Add-Fb2PromptLine -Lines $lines -Text ('- full_final_complete: `{0}`' -f [bool]$gates.full_final_complete)
    Add-Fb2PromptLine -Lines $lines -Text ('- token_present: `{0}`' -f [bool]$gates.token_present)
    Add-Fb2PromptLine -Lines $lines -Text ('- voice_deferred_by_user: `{0}`' -f [bool]$gates.voice_deferred_by_user)
    Add-Fb2PromptLine -Lines $lines -Text ('- next_minimum_action: `{0}`' -f [string]$gates.next_minimum_action)
    Add-Fb2PromptLine -Lines $lines -Text ('- totals: complete `{0}` / deferred `{1}` / incomplete `{2}` / total `{3}`' -f [int]$totals.complete, [int]$totals.deferred, [int]$totals.incomplete, [int]$totals.total)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## Owner 下一步"
    Add-Fb2PromptLine -Lines $lines -Text ('- main_project: `{0}`' -f [string]$ownerActions.main_project)
    Add-Fb2PromptLine -Lines $lines -Text ('- fb2_project: `{0}`' -f [string]$ownerActions.fb2_project)
    Add-Fb2PromptLine -Lines $lines -Text ('- shared: `{0}`' -f [string]$ownerActions.shared)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 可执行命令"
    foreach ($name in @("refresh_status", "read_status_refresh", "validate_gap_action_board", "validate_completion_matrix", "validate_handoff_prompt", "validate_visible_answer_policy", "no_write_direct_read", "data_only_preflight", "visible_regression_requires_authorization")) {
        $value = Protect-Fb2PromptSecret -Text ([string](Get-Fb2PromptProperty $commands $name ""))
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            Add-Fb2PromptLine -Lines $lines -Text ('- `{0}`: `{1}`' -f $name, $value)
        }
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 阻塞与边界"
    Add-Fb2PromptLine -Lines $lines -Text ('- external_secret: `{0}`' -f [string]$blocking.external_secret)
    Add-Fb2PromptLine -Lines $lines -Text ('- blocked_by_external_secret: `{0}`' -f [bool]$blocking.blocked_by_external_secret)
    Add-Fb2PromptLine -Lines $lines -Text ('- safe_to_continue_without_secret: `{0}`' -f (@($blocking.safe_to_continue_without_secret) -join ', '))
    Add-Fb2PromptLine -Lines $lines -Text ('- requires_secret: `{0}`' -f (@($blocking.requires_secret) -join ', '))
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 缺口行动板"
    Add-Fb2PromptLine -Lines $lines -Text ('- gap_schema: `{0}`' -f [string](Get-Fb2PromptProperty $gapBoard 'schema' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- action_count: `{0}`' -f [int](Get-Fb2PromptProperty $gapBoard 'action_count' 0))
    foreach ($action in $gapActions) {
        $actionId = Format-Fb2PromptCell $action.id 120
        $actionStatus = Format-Fb2PromptCell $action.status 100
        $actionOwner = Format-Fb2PromptCell $action.owner 80
        $actionEvidence = Format-Fb2PromptCell $action.evidence_needed 220
        $actionCommand = Format-Fb2PromptCell $action.command 220
        $actionNotes = Format-Fb2PromptCell $action.notes 220
        Add-Fb2PromptLine -Lines $lines -Text ("- gap {0}: status={1}; owner={2}; evidence={3}; command={4}; notes={5}" -f $actionId, $actionStatus, $actionOwner, $actionEvidence, $actionCommand, $actionNotes)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 证据新鲜度"
    Add-Fb2PromptLine -Lines $lines -Text ('- freshness_schema: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'schema' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- generated_at_utc: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'generated_at_utc' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- note: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'note' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- current_output_artifact_count: `{0}`' -f [int](Get-Fb2PromptProperty $freshness 'current_output_artifact_count' 0))
    Add-Fb2PromptLine -Lines $lines -Text ('- history_artifact_count: `{0}`' -f [int](Get-Fb2PromptProperty $freshness 'history_artifact_count' 0))
    $pipe = [char]124
    Add-Fb2PromptLine -Lines $lines -Text ('{0} artifact {0} source {0} age_minutes {0} path {0}' -f $pipe)
    Add-Fb2PromptLine -Lines $lines -Text ('{0}---{0}---{0}---:{0}---{0}' -f $pipe)
    foreach ($artifact in $freshnessArtifacts) {
        $name = Format-Fb2PromptCell $artifact.name 80
        $source = Format-Fb2PromptCell $artifact.source_scope 80
        $age = Format-Fb2PromptCell $artifact.age_minutes 40
        $path = Format-Fb2PromptCell $artifact.path 180
        Add-Fb2PromptLine -Lines $lines -Text ('{0} {1} {0} {2} {0} {3} {0} {4} {0}' -f $pipe, $name, $source, $age, $path)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 完成矩阵"
    Add-Fb2PromptLine -Lines $lines -Text ('{0} group {0} owner {0} id {0} status {0} evidence {0} missing {0}' -f $pipe)
    Add-Fb2PromptLine -Lines $lines -Text ('{0}---{0}---{0}---{0}---{0}---{0}---{0}' -f $pipe)
    foreach ($requirement in $requirements) {
        $group = Format-Fb2PromptCell $requirement.group 80
        $owner = Format-Fb2PromptCell $requirement.owner 80
        $id = Format-Fb2PromptCell $requirement.id 80
        $status = Format-Fb2PromptCell $requirement.status 80
        $evidence = Format-Fb2PromptCell $requirement.evidence 220
        $missing = Format-Fb2PromptCell $requirement.missing 160
        Add-Fb2PromptLine -Lines $lines -Text ('{0} {1} {0} {2} {0} {3} {0} {4} {0} {5} {0} {6} {0}' -f $pipe, $group, $owner, $id, $status, $evidence, $missing)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 接手规则"
    Add-Fb2PromptLine -Lines $lines -Text '- 先运行 `refresh_status`，再读取 `status-refresh-current.json`。'
    Add-Fb2PromptLine -Lines $lines -Text '- 没有 `FB2_AI_CENTER_TOKEN` 时，只做公开契约、离线样本、无写群直读和文档/脚本回归。'
    Add-Fb2PromptLine -Lines $lines -Text '- 有 token 后，先跑 `data_only_preflight`，刷新 live Context Pack、本人订单、平台摘要、权限和质量证据。'
    Add-Fb2PromptLine -Lines $lines -Text '- 真实群聊可见写入必须另有明确授权；截图不能替代 API 直读 summary。'
    Add-Fb2PromptLine -Lines $lines -Text '- ASR/TTS final evidence 仍按用户要求暂停，不能把 `full_final_complete=false` 改成完成。'

    return (($lines -join [Environment]::NewLine) + [Environment]::NewLine)
}

function Assert-Fb2PromptSelfTest {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw "SelfTest failed: $Message"
    }
}

function Invoke-Fb2PromptSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-handoff-prompt-selftest-" + [guid]::NewGuid().ToString("N"))
    $refreshPath = Join-Path $tempRoot "status-refresh-current.json"
    $promptPath = Join-Path $tempRoot "handoff-prompt-current.md"
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $fixture = [pscustomobject]@{
            schema = "fb2.main_project.status_refresh.v1"
            owner_next_actions = [ordered]@{
                main_project = "keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available"
                fb2_project = "provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence"
                shared = "run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json"
            }
            blocking_state = [ordered]@{
                blocked_by_external_secret = $true
                external_secret = "FB2_AI_CENTER_TOKEN"
                safe_to_continue_without_secret = @("status_refresh_selftest")
                requires_secret = @("live_context_pack_permission_quality_refresh")
            }
            next_commands = [ordered]@{
                refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
                read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
                validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
                validate_completion_matrix = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1"
                validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
                validate_visible_answer_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>"
                no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Password secret-real-password"
                data_only_preflight = '$env:FB2_AI_CENTER_TOKEN="secret-real-value"; pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Token secret-real-value'
                visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages"
            }
            completion_matrix = [ordered]@{
                schema = "fb2.main_project.completion_matrix.v1"
                totals = [ordered]@{ total = 2; complete = 1; deferred = 1; incomplete = 0 }
                gates = [ordered]@{
                    data_goal_complete = $true
                    full_final_complete = $false
                    token_present = $false
                    voice_deferred_by_user = $true
                    next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
                }
                requirements = @(
                    [ordered]@{ id = "today_matches_analysis"; group = "user_scenarios"; owner = "shared"; title = "today"; status = "complete"; complete = $true; deferred = $false; evidence = "sample"; missing = "" },
                    [ordered]@{ id = "voice_final_evidence"; group = "voice_deferred_by_user"; owner = "paused_by_user"; title = "voice"; status = "deferred"; complete = $false; deferred = $true; evidence = ""; missing = "ASR/TTS is intentionally deferred by user" }
                )
            }
            evidence_freshness = [ordered]@{
                schema = "fb2.main_project.evidence_freshness.v1"
                generated_at_utc = "2026-06-23T00:00:00.0000000Z"
                note = "artifact freshness only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN"
                current_output_artifact_count = 2
                history_artifact_count = 0
                artifacts = @(
                    [ordered]@{ name = "status"; source_scope = "current_output_dir"; age_minutes = 0; path = "target\fb2-ai-center\status-current.json" },
                    [ordered]@{ name = "goal_audit"; source_scope = "current_output_dir"; age_minutes = 0; path = "target\fb2-ai-center\goal-audit-current.json" }
                )
            }
            gap_action_board = [ordered]@{
                schema = "fb2.main_project.gap_action_board.v1"
                action_count = 2
                actions = @(
                    [ordered]@{
                        id = "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
                        status = "blocked_by_external_secret"
                        owner = "fb2_project_and_shared"
                        evidence_needed = "FB2_AI_CENTER_TOKEN or equivalent exported live Context Pack / permission / quality evidence"
                        command = '$env:FB2_AI_CENTER_TOKEN="secret-real-value"; pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Token secret-real-value'
                        notes = "Run no-write DataOnlyAcceptance preflight after token is available."
                    },
                    [ordered]@{
                        id = "voice_final_evidence"
                        status = "deferred_by_user"
                        owner = "paused_by_user"
                        evidence_needed = "real device ASR/TTS evidence"
                        command = ""
                        notes = "ASR/TTS is paused."
                    }
                )
            }
        }
        $fixture | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $refreshPath -Encoding UTF8
        & $PSCommandPath -RefreshPath $refreshPath -OutputPath $promptPath | Out-Null
        $content = Get-Content -LiteralPath $promptPath -Raw
        Assert-Fb2PromptSelfTest (Test-Path -LiteralPath $promptPath) "prompt file exists"
        Assert-Fb2PromptSelfTest ($content -match "fb2 AI Center") "prompt title"
        Assert-Fb2PromptSelfTest ($content -match "today_matches_analysis") "matrix item"
        Assert-Fb2PromptSelfTest ($content -match "证据新鲜度") "freshness section"
        Assert-Fb2PromptSelfTest ($content -match "fb2.main_project.evidence_freshness.v1") "freshness schema"
        Assert-Fb2PromptSelfTest ($content -match "缺口行动板") "gap action section"
        Assert-Fb2PromptSelfTest ($content -match "fb2.main_project.gap_action_board.v1") "gap action schema"
        Assert-Fb2PromptSelfTest ($content -match "validate_completion_matrix") "completion matrix validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_handoff_prompt") "handoff prompt validation command"
        Assert-Fb2PromptSelfTest ($content -match "<FB2_AI_CENTER_TOKEN>") "token placeholder"
        Assert-Fb2PromptSelfTest ($content -match "<FB2_PASSWORD>") "password placeholder"
        Assert-Fb2PromptSelfTest ($content -notmatch "secret-real-value") "token redacted"
        Assert-Fb2PromptSelfTest ($content -notmatch "secret-real-password") "password redacted"
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2PromptSelfTest
    exit 0
}

$root = Get-Fb2PromptRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2PromptPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\handoff-prompt-current.md"
} else {
    $OutputPath = Resolve-Fb2PromptPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2PromptJson -Path $RefreshPath
$prompt = New-Fb2HandoffPrompt -Refresh $refresh -SourcePath $RefreshPath
Set-Content -LiteralPath $OutputPath -Value $prompt -Encoding UTF8

[pscustomobject]@{
    schema = "fb2.main_project.handoff_prompt_result.v1"
    source_refresh = $RefreshPath
    output_path = $OutputPath
    requirement_count = @($refresh.completion_matrix.requirements).Count
} | ConvertTo-Json -Depth 4

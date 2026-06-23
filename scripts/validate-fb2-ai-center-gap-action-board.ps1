#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2GapRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2GapPath {
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

function Get-Fb2GapProperty {
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

function Read-Fb2GapJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2GapCheck {
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

function Test-Fb2GapSecretSafe {
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
    return $true
}

function Find-Fb2GapAction {
    param(
        [object[]]$Actions,
        [string]$Id
    )

    @($Actions | Where-Object { [string]$_.id -eq $Id } | Select-Object -First 1)
}

function New-Fb2GapValidation {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $board = Get-Fb2GapProperty $Refresh "gap_action_board"
    $actions = @(Get-Fb2GapProperty $board "actions" @())

    Add-Fb2GapCheck $checks "gap board schema" ([string](Get-Fb2GapProperty $board "schema" "") -eq "fb2.main_project.gap_action_board.v1")
    Add-Fb2GapCheck $checks "action count matches" ([int](Get-Fb2GapProperty $board "action_count" 0) -eq @($actions).Count) ("declared=$([int](Get-Fb2GapProperty $board 'action_count' 0)) actual=$(@($actions).Count)")
    Add-Fb2GapCheck $checks "has actions" (@($actions).Count -gt 0)

    foreach ($action in $actions) {
        $id = [string](Get-Fb2GapProperty $action "id" "")
        Add-Fb2GapCheck $checks "action $id has owner" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action "owner" "")))
        Add-Fb2GapCheck $checks "action $id has evidence_needed" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action "evidence_needed" "")))
        Add-Fb2GapCheck $checks "action $id command secret safe" (Test-Fb2GapSecretSafe -Text ([string](Get-Fb2GapProperty $action "command" "")))
        Add-Fb2GapCheck $checks "action $id notes secret safe" (Test-Fb2GapSecretSafe -Text ([string](Get-Fb2GapProperty $action "notes" "")))
    }

    $tokenAction = Find-Fb2GapAction -Actions $actions -Id "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
    Add-Fb2GapCheck $checks "token refresh action exists" (@($tokenAction).Count -gt 0)
    if (@($tokenAction).Count -gt 0) {
        $command = [string](Get-Fb2GapProperty $tokenAction[0] "command" "")
        Add-Fb2GapCheck $checks "token refresh action blocked by secret" ([string](Get-Fb2GapProperty $tokenAction[0] "status" "") -eq "blocked_by_external_secret")
        Add-Fb2GapCheck $checks "token refresh action no write group" (-not [bool](Get-Fb2GapProperty $tokenAction[0] "requires_visible_group_write" $true))
        Add-Fb2GapCheck $checks "token refresh action requires secret" (-not [bool](Get-Fb2GapProperty $tokenAction[0] "can_run_without_secret" $true))
        Add-Fb2GapCheck $checks "token refresh command is preflight" ($command -match "DataOnlyAcceptance" -and $command -match "PreflightOnly")
        Add-Fb2GapCheck $checks "token refresh command has placeholder" ($command -match "<FB2_AI_CENTER_TOKEN>")
    }

    foreach ($id in @("voice_final_evidence", "ASR_TTS_final_evidence")) {
        $action = Find-Fb2GapAction -Actions $actions -Id $id
        if (@($action).Count -gt 0) {
            $status = [string](Get-Fb2GapProperty $action[0] "status" "")
            Add-Fb2GapCheck $checks "$id is deferred" ($status -match "^deferred")
            Add-Fb2GapCheck $checks "$id owned by pause" ([string](Get-Fb2GapProperty $action[0] "owner" "") -eq "paused_by_user")
            Add-Fb2GapCheck $checks "$id has no command" ([string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action[0] "command" "")))
        }
    }

    $fullFinal = Find-Fb2GapAction -Actions $actions -Id "full_final_acceptance_same_batch_voice_and_visible_chat"
    if (@($fullFinal).Count -gt 0) {
        Add-Fb2GapCheck $checks "full final requires visible group write" ([bool](Get-Fb2GapProperty $fullFinal[0] "requires_visible_group_write" $false))
        Add-Fb2GapCheck $checks "full final waits on voice/authorization" ([string](Get-Fb2GapProperty $fullFinal[0] "status" "") -match "voice|visible")
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.gap_action_board_validation.v1"
        source_refresh = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }
}

function Invoke-Fb2GapSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-gap-action-selftest-" + [guid]::NewGuid().ToString("N"))
    $refreshPath = Join-Path $tempRoot "status-refresh-current.json"
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $fixture = [pscustomobject]@{
            gap_action_board = [ordered]@{
                schema = "fb2.main_project.gap_action_board.v1"
                action_count = 3
                actions = @(
                    [ordered]@{
                        id = "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
                        status = "blocked_by_external_secret"
                        owner = "fb2_project_and_shared"
                        evidence_needed = "service token"
                        command = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
                        notes = "no write"
                        can_run_without_secret = $false
                        requires_visible_group_write = $false
                        deferred_by_user = $false
                    },
                    [ordered]@{
                        id = "voice_final_evidence"
                        status = "deferred_by_user"
                        owner = "paused_by_user"
                        evidence_needed = "voice evidence"
                        command = ""
                        notes = "paused"
                        can_run_without_secret = $false
                        requires_visible_group_write = $false
                        deferred_by_user = $true
                    },
                    [ordered]@{
                        id = "full_final_acceptance_same_batch_voice_and_visible_chat"
                        status = "waiting_on_voice_and_authorized_visible_regression"
                        owner = "shared"
                        evidence_needed = "same batch final evidence"
                        command = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages"
                        notes = "requires explicit authorization"
                        can_run_without_secret = $false
                        requires_visible_group_write = $true
                        deferred_by_user = $false
                    }
                )
            }
        }
        $fixture | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $refreshPath -Encoding UTF8
        $validation = New-Fb2GapValidation -Refresh (Read-Fb2GapJson -Path $refreshPath) -SourcePath $refreshPath
        if (-not [bool]$validation.success) {
            $validation | ConvertTo-Json -Depth 8
            throw "SelfTest failed: gap validation fixture failed"
        }
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2GapSelfTest
    exit 0
}

$root = Get-Fb2GapRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2GapPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\gap-action-board-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2GapPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2GapJson -Path $RefreshPath
$result = New-Fb2GapValidation -Refresh $refresh -SourcePath $RefreshPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}

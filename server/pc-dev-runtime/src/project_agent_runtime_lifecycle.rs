pub(crate) fn agent_runtime_lifecycle_helpers() -> &'static str {
    r#"
function New-AgentRunId {
    $stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssfffZ')
    $suffix = [System.Guid]::NewGuid().ToString('N').Substring(0, 8)
    return "agent-$stamp-$PID-$suffix"
}

function Write-AgentRunEvent {
    param(
        [Parameter(Mandatory = $true)][string]$Type,
        [AllowNull()]$Data
    )
    if (-not $Script:AgentRunLogPath) { return }
    try {
        $event = [ordered]@{
            ts = (Get-Date).ToUniversalTime().ToString('o')
            run_id = $Script:AgentRunId
            type = $Type
            data = $Data
        }
        $line = $event | ConvertTo-Json -Depth 20 -Compress
        Add-Content -LiteralPath $Script:AgentRunLogPath -Value $line -Encoding UTF8
    } catch {
        Write-Warning "agent lifecycle log write failed: $($_.Exception.Message)"
    }
}

function Initialize-AgentRunLifecycle {
    param([Parameter(Mandatory = $true)][string]$Label)
    if ($Script:AgentRunLogPath) { return }

    $id = $RunId.Trim()
    if (-not $id) {
        $id = New-AgentRunId
    }
    $safeId = ($id -replace '[^A-Za-z0-9_.-]', '_')
    $logDir = Join-Path $ProjectRoot '.elon\agent-runs'
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null

    $Script:AgentRunId = $safeId
    $Script:AgentRunLogPath = Join-Path $logDir "$safeId.jsonl"
    $Script:AgentRunLifecycleClosed = $false
    Remove-Item -LiteralPath $Script:AgentRunLogPath -Force -ErrorAction SilentlyContinue

    Write-Host "[run] $Script:AgentRunId"
    Write-Host "[run-log] $Script:AgentRunLogPath"
    Write-AgentRunEvent -Type 'run_started' -Data ([ordered]@{
        mode = $Label
        prompt_chars = $Prompt.Length
        max_turns = $MaxTurns
        max_run_commands = $MaxRunCommands
        dry_run = [bool]$DryRun
        auto_approve = [bool]$Yes
    })
}

function Get-AgentActionTarget {
    param([Parameter(Mandatory = $true)]$Action)
    $tool = [string]$Action.tool
    switch ($tool) {
        'list_dir' { return (Limit-AgentText ([string]$Action.path) 300) }
        'read_file' { return (Limit-AgentText ([string]$Action.path) 300) }
        'read_file_range' {
            $target = "{0}:{1}+{2}" -f [string]$Action.path, [int]$Action.start_line, [int]$Action.line_count
            return (Limit-AgentText $target 300)
        }
        'write_file' { return (Limit-AgentText ([string]$Action.path) 300) }
        'apply_patch' {
            $patchText = [string]$Action.patch
            return "patch_chars=$($patchText.Length) check_only=$([bool]$Action.check_only)"
        }
        'run_command' {
            $program = [string]$Action.program
            if ($program.Trim()) {
                $args = @()
                if ($Action.args) {
                    $args = @($Action.args | ForEach-Object { [string]$_ })
                }
                return (Limit-AgentText "$program $($args -join ' ')" 300)
            }
            return (Limit-AgentText ([string]$Action.command) 300)
        }
        default { return '' }
    }
}

function Complete-AgentRunLifecycle {
    param(
        [ValidateSet('completed', 'failed')][string]$Status = 'completed',
        [AllowNull()]$Data
    )
    if ($Script:AgentRunLifecycleClosed) { return }
    $payload = [ordered]@{
        status = $Status
        run_commands_used = $Script:AgentRunCommandCount
    }
    if ($null -ne $Data) {
        $payload.details = $Data
    }
    $eventType = if ($Status -eq 'failed') { 'run_failed' } else { 'run_finished' }
    Write-AgentRunEvent -Type $eventType -Data $payload
    $Script:AgentRunLifecycleClosed = $true
}
"#
}

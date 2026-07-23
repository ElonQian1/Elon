$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-activation.ps1')

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message (actual=$Actual expected=$Expected)"
    }
}

foreach ($moduleName in @(
    'publish-node-agent.ps1',
    'node-agent-publish-handshake.ps1',
    'node-agent-release-outbox.ps1',
    'node-agent-remote-release-worker.ps1',
    'node-agent-local-activation.ps1',
    'node-agent-post-terminal-activator.ps1'
)) {
    $tokens = $null
    $parseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        (Join-Path $PSScriptRoot $moduleName), [ref]$tokens, [ref]$parseErrors
    ) | Out-Null
    Assert-Equal $parseErrors.Count 0 "PowerShell parser must accept $moduleName in the current runtime"
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-local-first-release-test-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $artifactRoot = Join-Path $root 'input'
    New-Item -ItemType Directory -Path $artifactRoot | Out-Null
    $exe = Join-Path $artifactRoot 'elon-pc-node.exe'
    $zip = Join-Path $artifactRoot 'elon-node-agent-windows.zip'
    [System.IO.File]::WriteAllBytes($exe, [byte[]](1,2,3,4))
    [System.IO.File]::WriteAllBytes($zip, [byte[]](5,6,7,8))
    $sha = 'a' * 40
    $identity = "0.3.69+$sha"
    $outbox = Join-Path $root 'outbox'

    $first = Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outbox -GitSha $sha `
        -Version '0.3.69' -ReleaseIdentity $identity -Changelog '中文离线发布 fixture' `
        -WindowsExe $exe -WindowsClientPackage $zip -GitCommonDir (Join-Path $root 'git')
    $duplicate = Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outbox -GitSha $sha `
        -Version '0.3.69' -ReleaseIdentity $identity -Changelog '中文离线发布 fixture' `
        -WindowsExe $exe -WindowsClientPackage $zip -GitCommonDir (Join-Path $root 'git')
    Assert-Equal $first.EventPath $duplicate.EventPath 'duplicate delivery must coalesce by immutable SHA'
    Assert-True (-not $duplicate.Created) 'duplicate delivery must not create a second event'
    Assert-Equal @(Get-ChildItem (Join-Path $outbox 'events') -Directory).Count 1 'outbox must contain one event'

    $raw = [System.IO.File]::ReadAllText($first.EventPath, [System.Text.Encoding]::UTF8)
    foreach ($secret in @('admin_token','authorization','password','credential','bearer')) {
        Assert-True (-not $raw.ToLowerInvariant().Contains($secret)) "outbox must not persist $secret"
    }
    $event = $raw | ConvertFrom-Json
    Assert-Equal $event.sync_state 'pending' 'new event must start pending'
    Assert-Equal $event.changelog '中文离线发布 fixture' 'PS5.1 must round-trip UTF-8 outbox JSON explicitly'
    Assert-True $event.local_result_independent 'remote state must not own the local activation result'

    $blackhole = Complete-NodeAgentRemoteReleaseAttempt -EventPath $first.EventPath `
        -Outcome retry -ErrorCode 'timeout' -NowMs 1000 -MaxAttempts 4
    Assert-Equal $blackhole.sync_state 'retrying' 'blackhole must remain retrying'
    Assert-True ($blackhole.next_attempt_at_ms -gt 1000) 'retry must use bounded backoff'
    Assert-True $blackhole.local_result_independent 'remote timeout must not reverse local success'

    $latencyPath = Join-Path $outbox 'events\latency\event.json'
    New-Item -ItemType Directory -Path (Split-Path $latencyPath) -Force | Out-Null
    Copy-Item -LiteralPath $first.EventPath -Destination $latencyPath
    $latency = Complete-NodeAgentRemoteReleaseAttempt -EventPath $latencyPath `
        -Outcome retry -ErrorCode 'high_latency_timeout' -NowMs 1500 -MaxAttempts 4
    Assert-Equal $latency.sync_state 'retrying' 'high latency must remain a retryable remote condition'

    $reloaded = Read-NodeAgentRemoteReleaseEvent -EventPath $first.EventPath
    Assert-Equal $reloaded.sync_state 'retrying' 'restart must recover durable retry state'
    $recovered = Complete-NodeAgentRemoteReleaseAttempt -EventPath $first.EventPath `
        -Outcome success -NowMs $reloaded.next_attempt_at_ms -MaxAttempts 4
    Assert-Equal $recovered.sync_state 'synced' 'network recovery must converge to synced'
    Assert-Equal $recovered.attempt_count 2 'recovery must preserve attempt history'

    $failedPath = Join-Path $outbox 'events\failed\event.json'
    New-Item -ItemType Directory -Path (Split-Path $failedPath) -Force | Out-Null
    Copy-Item -LiteralPath $first.EventPath -Destination $failedPath
    for ($attempt = 0; $attempt -lt 4; $attempt++) {
        $failed = Complete-NodeAgentRemoteReleaseAttempt -EventPath $failedPath `
            -Outcome retry -ErrorCode 'offline' -NowMs (2000 + $attempt) -MaxAttempts 4
    }
    Assert-Equal $failed.sync_state 'failed' 'finite retries must end in failed'
    Assert-True $failed.local_result_independent 'remote exhaustion must not reverse local success'

    $activationRoot = Join-Path $root 'activation'
    $old = Register-NodeAgentVerifiedLocalRelease -StateRoot $activationRoot `
        -GitSha ('b' * 40) -Version '0.3.69' -ReleaseIdentity ("0.3.69+" + ('b' * 40)) `
        -WindowsClientPackage $zip -WindowsClientSha256 ((Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()) `
        -VerifiedAtMs 100
    $new = Register-NodeAgentVerifiedLocalRelease -StateRoot $activationRoot `
        -GitSha ('c' * 40) -Version '0.3.69' -ReleaseIdentity ("0.3.69+" + ('c' * 40)) `
        -WindowsClientPackage $zip -WindowsClientSha256 ((Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()) `
        -VerifiedAtMs 200
    $oldState = Get-Content -Raw -LiteralPath $old.StatePath | ConvertFrom-Json
    Assert-Equal $oldState.activation_state 'superseded' 'new verified release must supersede stale scheduled target'
    Assert-Equal $oldState.superseded_by $new.ReleaseIdentity 'supersede must name exact newer release'
    $selected = Get-LatestNodeAgentVerifiedLocalRelease -StateRoot $activationRoot
    Assert-Equal $selected.release_identity $new.ReleaseIdentity 'activator must select latest verified identity'

    $gate = Test-NodeAgentActivationOwnerGate -Status ([pscustomobject]@{
        active_cli_prompt_count = 1
        active_cli_prompt_task_ids = @('local-live')
    })
    Assert-True (-not $gate.Safe) 'live owner must block activation'
    $safeGate = Test-NodeAgentActivationOwnerGate -Status ([pscustomobject]@{
        active_cli_prompt_count = 0
        active_cli_prompt_task_ids = @()
    })
    Assert-True $safeGate.Safe 'zero live owners must allow activation'

    $success = Invoke-NodeAgentActivationTransaction -Release $selected `
        -OwnerGate { [pscustomobject]@{ Safe = $true; Reason = 'fixture' } } `
        -Apply { param($release) [pscustomobject]@{ Applied = $true; Backup = 'fixture-backup' } } `
        -Health { param($release) $true } `
        -Rollback { throw 'rollback must not run on success' }
    Assert-Equal $success.activation_state 'activated' 'healthy activation must commit'

    $rolledBack = $false
    $rollback = Invoke-NodeAgentActivationTransaction -Release $selected `
        -OwnerGate { [pscustomobject]@{ Safe = $true; Reason = 'fixture' } } `
        -Apply { param($release) [pscustomobject]@{ Applied = $true; Backup = 'fixture-backup' } } `
        -Health { param($release) $false } `
        -Rollback { param($release, $applyResult) $script:rolledBack = $true }
    Assert-Equal $rollback.activation_state 'rolled_back' 'health failure must roll back'
    Assert-True $script:rolledBack 'rollback callback must run'

    $publishText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'publish-node-agent.ps1')
    Assert-True ($publishText.Contains('[switch]$SynchronousRemote')) 'remote work must require explicit worker mode'
    Assert-True ($publishText.Contains('NODE_AGENT_LOCAL_PREPARE_STATUS=complete')) 'default publish must expose completed local preparation'
    Assert-True ($publishText.Contains('NODE_AGENT_LOCAL_ACTIVATION_STATUS=restart_scheduled')) `
        'publisher must not claim activation before the post-terminal safety gate passes'
    Assert-True ($publishText.Contains('NODE_AGENT_REMOTE_SYNC_STATE=pending')) 'default publish must expose async remote state'
    $validationText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'validate-node-agent-local-first.ps1')
    Assert-Equal ([regex]::Matches($validationText, "validate-rust\.ps1").Count) 1 `
        'targeted Rust filters must share exactly one validate-rust invocation'
    Assert-True ($validationText.Contains("-Domain','node-agent-local-first-v1'")) `
        'targeted Rust validation must share one stable verified cache domain'
    Assert-True ($validationText.Contains('NODE_AGENT_VALIDATION_CARGO_CACHE_REUSED=')) `
        'targeted validation must report cache hit evidence'
    $workerText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'node-agent-remote-release-worker.ps1')
    Assert-True ($workerText.Contains('WaitForExit($AttemptTimeoutSeconds * 1000)')) `
        'each remote attempt must have a finite process timeout'
    Assert-True ($workerText.Contains('Resolve-EventSourceWorktree')) `
        'worker restart must reconstruct the immutable source worktree from durable state'

    Write-Host 'NODE_AGENT_LOCAL_FIRST_TESTS=passed'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

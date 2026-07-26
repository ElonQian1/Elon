$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-activation.ps1')
. (Join-Path $PSScriptRoot 'node-agent-release-build-cache.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-rollback.ps1')
. (Join-Path $PSScriptRoot 'node-agent-release-contract.ps1')

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

function Assert-Throws {
    param([scriptblock]$Action, [string]$Pattern, [string]$Message)
    $caught = $null
    try {
        & $Action
    } catch {
        $caught = $_
    }
    if ($null -eq $caught) { throw "$Message (no exception was thrown)" }
    if (-not [string]::IsNullOrWhiteSpace($Pattern) -and
        $caught.Exception.Message -notmatch $Pattern) {
        throw "$Message (actual=$($caught.Exception.Message) expected_pattern=$Pattern)"
    }
}

foreach ($moduleName in @(
    'publish-node-agent.ps1',
    'node-agent-publish-handshake.ps1',
    'node-agent-release-outbox.ps1',
    'node-agent-release-build-cache.ps1',
    'node-agent-release-packaging.ps1',
    'node-agent-windows-installer.ps1',
    'node-agent-remote-release-worker.ps1',
    'node-agent-local-activation.ps1',
    'node-agent-local-rollback.ps1',
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
    $metadata = Get-NodeAgentCargoMetadata -ManifestPath (Join-Path $RepoRoot 'server\Cargo.toml')
    Assert-Equal $metadata.packages[0].name 'elon-server' `
        'cargo metadata must remain valid UTF-8 JSON under a non-ASCII repository path'
    Assert-True ([string]$metadata.target_directory -ne '') `
        'UTF-8 cargo metadata must expose the resolved target directory'
    $gitCommonDir = Get-NodeAgentGitCommonDir -RepoRoot $RepoRoot
    Assert-True (Test-Path -LiteralPath $gitCommonDir -PathType Container) `
        'Git common directory must remain a valid path under a non-ASCII repository path'
    Assert-True ($gitCommonDir.EndsWith('.git', [System.StringComparison]::OrdinalIgnoreCase)) `
        'Git common directory must resolve to the shared repository metadata root'

    $artifactRoot = Join-Path $root 'input'
    New-Item -ItemType Directory -Path $artifactRoot | Out-Null
    $exe = Join-Path $artifactRoot 'elon-pc-node.exe'
    $zip = Join-Path $artifactRoot 'elon-node-agent-windows.zip'
    $installer = Join-Path $artifactRoot 'elon-node-agent-windows-setup.exe'
    [System.IO.File]::WriteAllBytes($exe, [byte[]](1,2,3,4))
    [System.IO.File]::WriteAllBytes($zip, [byte[]](5,6,7,8))
    [System.IO.File]::WriteAllBytes($installer, [byte[]](9,10,11,12))
    $sha = 'a' * 40
    $identity = "0.3.69+$sha"
    $outbox = Join-Path $root 'outbox'

    $first = Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outbox -GitSha $sha `
        -Version '0.3.69' -ReleaseIdentity $identity -Changelog '中文离线发布 fixture' `
        -WindowsExe $exe -WindowsClientPackage $zip -WindowsInstallerPackage $installer `
        -GitCommonDir (Join-Path $root 'git')
    $duplicate = Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outbox -GitSha $sha `
        -Version '0.3.69' -ReleaseIdentity $identity -Changelog '中文离线发布 fixture' `
        -WindowsExe $exe -WindowsClientPackage $zip -WindowsInstallerPackage $installer `
        -GitCommonDir (Join-Path $root 'git')
    Assert-Equal $first.EventPath $duplicate.EventPath 'duplicate delivery must coalesce by immutable SHA'
    Assert-True (-not $duplicate.Created) 'duplicate delivery must not create a second event'
    Assert-Equal @(Get-ChildItem (Join-Path $outbox 'events') -Directory).Count 1 'outbox must contain one event'

    $lockedArtifact = New-Object System.IO.FileStream(
        $exe,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::ReadWrite,
        [System.IO.FileShare]::None
    )
    try {
        Assert-Throws {
            Get-NodeAgentFileSha256 -Path $exe -Attempts 2 -RetryDelayMilliseconds 5 | Out-Null
        } 'bounded attempts' 'an exclusively locked release artifact must fail after a bounded retry window'
    } finally {
        $lockedArtifact.Dispose()
    }
    Assert-True ((Get-NodeAgentFileSha256 -Path $exe) -match '^[0-9a-f]{64}$') `
        'release artifact hashing must recover after a transient Windows lock is released'

    $raw = [System.IO.File]::ReadAllText($first.EventPath, [System.Text.Encoding]::UTF8)
    foreach ($secret in @('admin_token','authorization','password','credential','bearer')) {
        Assert-True (-not $raw.ToLowerInvariant().Contains($secret)) "outbox must not persist $secret"
    }
    $event = $raw | ConvertFrom-Json
    Assert-Equal $event.sync_state 'pending' 'new event must start pending'
    Assert-True (-not [bool]$event.include_linux) 'Linux release must be opt-in'
    Assert-Equal $event.changelog '中文离线发布 fixture' 'PS5.1 must round-trip UTF-8 outbox JSON explicitly'
    Assert-True $event.local_result_independent 'remote state must not own the local activation result'
    Assert-Equal $event.artifacts.windows_installer_sha256 `
        (Get-NodeAgentFileSha256 -Path $installer) `
        'outbox must persist the immutable Windows installer SHA-256'
    $linuxIntentConflict = $false
    try {
        Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outbox -GitSha $sha `
            -Version '0.3.69' -ReleaseIdentity $identity -Changelog '中文离线发布 fixture' `
            -WindowsExe $exe -WindowsClientPackage $zip -WindowsInstallerPackage $installer `
            -GitCommonDir (Join-Path $root 'git') `
            -IncludeLinux | Out-Null
    } catch {
        $linuxIntentConflict = $_.Exception.Message.Contains('different immutable identity')
    }
    Assert-True $linuxIntentConflict 'an immutable outbox event must not silently change Linux intent'

    $headSha = [string](& git -C $RepoRoot rev-parse HEAD)
    & cmd.exe /d /c exit 9
    Assert-Equal $LASTEXITCODE 9 'fixture must seed a stale native exit code'
    $desktopTreeHash = Get-NodeAgentReleaseInputHash -RepoRoot $RepoRoot -GitSha $headSha.Trim() `
        -GitPaths @('desktop-shell/src-tauri') -ToolVersions @('rustc fixture') `
        -EnvironmentValues @('ELON_FIXTURE=1')
    Assert-True ($desktopTreeHash -match '^[0-9a-f]{64}$') `
        'PS5.1 must replace a stale LASTEXITCODE when hashing a valid Git tree'
    Assert-Throws {
        Get-NodeAgentReleaseInputHash -RepoRoot $RepoRoot -GitSha $headSha.Trim() `
            -GitPaths @('missing-release-cache-fixture') | Out-Null
    } 'Cannot calculate release cache input' `
        'release cache input hashing must still fail closed for a missing Git path'

    $cacheRoot = Join-Path $root 'build-cache'
    $cachedFileOutput = Join-Path $root 'build-output\desktop.exe'
    $script:fileBuildCount = 0
    $firstFileCache = @(Invoke-NodeAgentCachedFileBuild -Kind 'desktop' -InputHash ('1' * 64) `
        -CacheRoot $cacheRoot -OutputPath $cachedFileOutput -Build {
            $script:fileBuildCount += 1
            New-Item -ItemType Directory -Force -Path (Split-Path $cachedFileOutput -Parent) | Out-Null
            [System.IO.File]::WriteAllBytes($cachedFileOutput, [byte[]](9,8,7))
        })
    Remove-Item -LiteralPath $cachedFileOutput -Force
    $secondFileCache = @(Invoke-NodeAgentCachedFileBuild -Kind 'desktop' -InputHash ('1' * 64) `
        -CacheRoot $cacheRoot -OutputPath $cachedFileOutput -Build {
            $script:fileBuildCount += 1
            throw 'cache hit must not rebuild'
        })
    Assert-Equal $script:fileBuildCount 1 'same immutable desktop inputs must build exactly once'
    Assert-True ($secondFileCache -contains 'NODE_AGENT_BUILD_CACHE_HIT=true') `
        'desktop cache hit must emit machine-readable evidence'

    $cachedDirectoryOutput = Join-Path $root 'pc-dist'
    $script:directoryBuildCount = 0
    Invoke-NodeAgentCachedDirectoryBuild -Kind 'pc-frontend' -InputHash ('2' * 64) `
        -CacheRoot $cacheRoot -OutputDirectory $cachedDirectoryOutput -RequiredRelativePath 'index.html' `
        -Build {
            $script:directoryBuildCount += 1
            New-Item -ItemType Directory -Force -Path $cachedDirectoryOutput | Out-Null
            [System.IO.File]::WriteAllText((Join-Path $cachedDirectoryOutput 'index.html'), 'cached')
        } | Out-Null
    Remove-Item -LiteralPath $cachedDirectoryOutput -Recurse -Force
    $directoryHit = @(Invoke-NodeAgentCachedDirectoryBuild -Kind 'pc-frontend' -InputHash ('2' * 64) `
        -CacheRoot $cacheRoot -OutputDirectory $cachedDirectoryOutput -RequiredRelativePath 'index.html' `
        -Build {
            $script:directoryBuildCount += 1
            throw 'cache hit must not rebuild'
        })
    Assert-Equal $script:directoryBuildCount 1 'same immutable PC frontend inputs must build exactly once'
    Assert-True ($directoryHit -contains 'NODE_AGENT_BUILD_CACHE_HIT=true') `
        'PC frontend cache hit must emit machine-readable evidence'

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

    $installRoot = Join-Path $root 'installed\ElonNode'
    $internalRoot = Join-Path $installRoot '_internal'
    $pcAssetRoot = Join-Path $internalRoot 'pc-next-dist\assets'
    New-Item -ItemType Directory -Path $pcAssetRoot -Force | Out-Null
    $stableFiles = [ordered]@{
        '一龙开发平台.exe' = 'old-client'
        '卸载一龙开发平台.exe' = 'old-uninstaller'
        '_internal\elon-desktop.exe' = 'old-node'
        '_internal\node-agent-version.json' = '{"release_identity":"old"}'
        '_internal\node-agent.env' = 'ELON_DESKTOP_REVIEW_PUBLIC_KEYS_JSON=[]'
        '_internal\README.txt' = 'old-readme'
        '_internal\pc-next-dist\index.html' = '<html>old</html>'
        '_internal\pc-next-dist\assets\app.js' = 'old-app'
    }
    foreach ($relative in $stableFiles.Keys) {
        $path = Join-Path $installRoot $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force | Out-Null
        [System.IO.File]::WriteAllText($path, [string]$stableFiles[$relative], [System.Text.Encoding]::UTF8)
    }
    $deepRuntime = Join-Path $installRoot (
        'terminal-finalization-receipts-v1\' +
        ('receipt-segment-' * 4) +
        '\receipt.json'
    )
    New-Item -ItemType Directory -Path (Split-Path -Parent $deepRuntime) -Force | Out-Null
    [System.IO.File]::WriteAllText($deepRuntime, '{"runtime":"must-not-copy"}', [System.Text.Encoding]::UTF8)
    $finishContract = Join-Path $installRoot 'ai-finish-contracts-v1\active\contract.json'
    New-Item -ItemType Directory -Path (Split-Path -Parent $finishContract) -Force | Out-Null
    [System.IO.File]::WriteAllText($finishContract, '{"runtime":"must-not-copy"}', [System.Text.Encoding]::UTF8)
    $runtimeLog = Join-Path $internalRoot 'logs\nested\client-launcher.jsonl'
    New-Item -ItemType Directory -Path (Split-Path -Parent $runtimeLog) -Force | Out-Null
    [System.IO.File]::WriteAllText($runtimeLog, 'runtime-log', [System.Text.Encoding]::UTF8)
    [System.IO.File]::WriteAllText(
        (Join-Path $installRoot 'supervisor-node-url.txt'),
        'http://127.0.0.1:7800',
        [System.Text.Encoding]::UTF8
    )
    [System.IO.File]::WriteAllBytes((Join-Path $internalRoot 'watchdog.instance.lock'), [byte[]]@())

    $snapshotRoot = Join-Path $activationRoot 'rollback\fixture-valid'
    $snapshot = New-NodeAgentRollbackSnapshot -InstallRoot $installRoot `
        -SnapshotRoot $snapshotRoot -PriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $snapshot.FileCount $stableFiles.Count 'snapshot must contain every stable allowlisted file'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $snapshot.ClientRoot 'terminal-finalization-receipts-v1'))) `
        'deep terminal receipts must never enter the rollback client tree'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $snapshot.ClientRoot 'ai-finish-contracts-v1'))) `
        'finish contracts must never enter the rollback client tree'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $snapshot.ClientRoot '_internal\logs'))) `
        'runtime logs must never enter the rollback client tree'
    $verifiedSnapshot = Test-NodeAgentRollbackSnapshot -SnapshotRoot $snapshotRoot `
        -ExpectedPriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $verifiedSnapshot.ManifestSha256 $snapshot.ManifestSha256 `
        'snapshot must verify against its durable manifest hash'

    $corruptPath = Join-Path $snapshot.ClientRoot '_internal\elon-desktop.exe'
    [System.IO.File]::AppendAllText($corruptPath, 'corrupt', [System.Text.Encoding]::UTF8)
    Assert-Throws {
        Test-NodeAgentRollbackSnapshot -SnapshotRoot $snapshotRoot `
            -ExpectedPriorReleaseIdentity ("0.3.69+" + ('b' * 40)) | Out-Null
    } 'mismatch' 'corrupt rollback client content must fail closed'

    $unknownPath = Join-Path $internalRoot 'unexpected-client-runtime.dll'
    [System.IO.File]::WriteAllText($unknownPath, 'unknown', [System.Text.Encoding]::UTF8)
    Assert-Throws {
        New-NodeAgentRollbackSnapshot -InstallRoot $installRoot `
            -SnapshotRoot (Join-Path $activationRoot 'rollback\fixture-unknown') `
            -PriorReleaseIdentity ("0.3.69+" + ('b' * 40)) | Out-Null
    } 'unclassified' 'unknown stable client files must require an explicit allowlist decision'
    Remove-Item -LiteralPath $unknownPath -Force

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
        -Prepare {
            param($release)
            [pscustomobject]@{
                SnapshotRoot = $snapshotRoot
                SnapshotManifestSha256 = $snapshot.ManifestSha256
            }
        } `
        -Apply { param($release, $prepared) [pscustomobject]@{ Applied = $true } } `
        -Health { param($release) $true } `
        -Rollback { throw 'rollback must not run on success' } `
        -PriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $success.activation_state 'activated' 'healthy activation must commit'
    Assert-Equal $success.receipt.snapshot_manifest_sha256 $snapshot.ManifestSha256 `
        'activation receipt must bind the verified rollback manifest'
    $receiptPath = Join-Path (Split-Path -Parent $selected.state_path) 'activation-receipt.json'
    Assert-True (Test-Path -LiteralPath $receiptPath -PathType Leaf) `
        'activation result must persist a standalone receipt'
    $persistedReceipt = [System.IO.File]::ReadAllText($receiptPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    Assert-Equal $persistedReceipt.outcome 'activated' 'standalone activation receipt must preserve the outcome'

    $rolledBack = $false
    $rollback = Invoke-NodeAgentActivationTransaction -Release $selected `
        -OwnerGate { [pscustomobject]@{ Safe = $true; Reason = 'fixture' } } `
        -Prepare {
            param($release)
            [pscustomobject]@{
                SnapshotRoot = $snapshotRoot
                SnapshotManifestSha256 = $snapshot.ManifestSha256
            }
        } `
        -Apply { param($release, $prepared) [pscustomobject]@{ Applied = $true } } `
        -Health { param($release) $false } `
        -Rollback {
            param($release, $prepared, $applyResult)
            Assert-Equal $prepared.SnapshotManifestSha256 $snapshot.ManifestSha256 `
                'rollback must receive the verified pre-apply context'
            $script:rolledBack = $true
        } `
        -PriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $rollback.activation_state 'rolled_back' 'health failure must roll back'
    Assert-True $script:rolledBack 'rollback callback must run'
    Assert-Equal $rollback.receipt.rollback_state 'succeeded' `
        'rollback receipt must record a verified successful rollback'

    $applyRollback = $false
    $applyFailure = Invoke-NodeAgentActivationTransaction -Release $selected `
        -OwnerGate { [pscustomobject]@{ Safe = $true; Reason = 'fixture' } } `
        -Prepare {
            param($release)
            [pscustomobject]@{
                SnapshotRoot = $snapshotRoot
                SnapshotManifestSha256 = $snapshot.ManifestSha256
            }
        } `
        -Apply { param($release, $prepared) throw 'fixture repair failed after snapshot' } `
        -Health { param($release) throw 'health must not run after apply failure' } `
        -Rollback {
            param($release, $prepared, $applyResult)
            Assert-Equal $prepared.SnapshotRoot $snapshotRoot `
                'apply failure must retain the verified rollback context'
            $script:applyRollback = $true
        } `
        -PriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $applyFailure.activation_state 'rolled_back' `
        'repair entrypoint failure after snapshot must roll back'
    Assert-True $script:applyRollback 'repair entrypoint failure must invoke rollback'

    $prepareApplied = $false
    $prepareRolledBack = $false
    $prepareFailure = Invoke-NodeAgentActivationTransaction -Release $selected `
        -OwnerGate { [pscustomobject]@{ Safe = $true; Reason = 'fixture' } } `
        -Prepare { throw 'fixture snapshot verification failed' } `
        -Apply { $script:prepareApplied = $true } `
        -Health { $true } `
        -Rollback { $script:prepareRolledBack = $true } `
        -PriorReleaseIdentity ("0.3.69+" + ('b' * 40))
    Assert-Equal $prepareFailure.activation_state 'failed' `
        'snapshot preparation failure must stop before mutation'
    Assert-Equal $prepareFailure.receipt.rollback_state 'not_required' `
        'pre-apply failure must not pretend to roll back'
    Assert-True (-not $script:prepareApplied) 'pre-apply failure must not run the installer'
    Assert-True (-not $script:prepareRolledBack) 'pre-apply failure must not restart the old client'

    $publishText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'publish-node-agent.ps1')
    Assert-True ($publishText.Contains('[switch]$SynchronousRemote')) 'remote work must require explicit worker mode'
    Assert-True ($publishText.Contains('[switch]$IncludeLinux')) 'Linux publishing must require an explicit switch'
    Assert-True ($publishText.Contains("if (`$SynchronousRemote -and `$IncludeLinux)")) `
        'remote outbox must not build Linux unless explicitly requested'
    Assert-True ($publishText.Contains("Invoke-RustCacheCargo -ProjectRoot `$RepoRoot -Domain 'node-agent-release'")) `
        'release Cargo builds must use the managed shared-cache runtime'
    Assert-True ($publishText.Contains('NODE_AGENT_LOCAL_PREPARE_STATUS=complete')) 'default publish must expose completed local preparation'
    Assert-True ($publishText.Contains('NODE_AGENT_LOCAL_ACTIVATION_STATUS=restart_scheduled')) `
        'publisher must not claim activation before the post-terminal safety gate passes'
    Assert-True ($publishText.Contains('NODE_AGENT_REMOTE_SYNC_STATE=pending')) 'default publish must expose async remote state'
    Assert-True ($publishText.Contains("'node-agent-remote-publish-v1.lock'") -and `
        $publishText.Contains("'node-agent-local-publish-v1.lock'")) `
        'remote retry lane must never hold the local publish lock'
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
    Assert-True ($workerText.Contains('[int]$AttemptTimeoutSeconds = 600')) `
        'one stalled remote attempt must not serialize the durable outbox for an hour'
    $outboxText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
    Assert-True ($outboxText.Contains('[int]$AttemptTimeoutSeconds = 600')) `
        'the outbox launcher must not override the worker with the legacy one-hour timeout'
    Assert-True ($publishText.Contains("--connect-timeout 5 --max-time 20 -fsS -X POST")) `
        'remote update broadcast must have a bounded response window'
    Assert-True ($publishText.Contains('if [ "$curl_status" -eq 28 ]')) `
        'a timed-out non-repeatable broadcast must continue to bounded handshake verification'
    Assert-True ($workerText.Contains('Resolve-EventSourceWorktree')) `
        'worker restart must reconstruct the immutable source worktree from durable state'
    Assert-True ($workerText.Contains("`$arguments += '-IncludeLinux'")) `
        'the worker must preserve explicit Linux intent from the durable event'
    Assert-True ($workerText.Contains('$gitExit = $LASTEXITCODE')) `
        'PS5.1 worker must judge git worktree recovery by exit code instead of stderr progress'

    Write-Host 'NODE_AGENT_LOCAL_FIRST_TESTS=passed'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

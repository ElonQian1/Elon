$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& git -C $RepoRoot rev-parse --is-inside-work-tree *> $null
if ($LASTEXITCODE -ne 0) { throw "Unable to validate repository root." }
$Modules = Join-Path $RepoRoot "scripts\validation"
Import-Module (Join-Path $Modules "Validation.Arguments.psm1") -Force -DisableNameChecking
$parsed = Split-ValidationCargoArguments -Arguments $args -ValueOptions @{
    '-CacheRoot'='CacheRoot'; '-Domain'='Domain'; '-TargetDir'='TargetDir'; '-WaitTimeoutSeconds'='WaitTimeoutSeconds'; '-LightSlots'='LightSlots'
} -SwitchOptions @('-Force','-DisableSccache','-SkipCheapGates')
$CacheRoot = $parsed.wrapper.CacheRoot
$Domain = if ($parsed.wrapper.Domain) { $parsed.wrapper.Domain } else { 'agent-validation' }
$requestedDomain = $Domain
Import-Module (Join-Path $RepoRoot "scripts\rust-cache\RustCache.Policy.psm1") -Force -DisableNameChecking
$Domain = Resolve-RustCacheDomain -ProjectRoot $RepoRoot -Domain $Domain
if ($requestedDomain -ne $Domain) { Write-Host "RUST_CACHE_DOMAIN_CANONICALIZED=$requestedDomain->$Domain" }
$TargetDir = $parsed.wrapper.TargetDir
$WaitTimeoutSeconds = if ($parsed.wrapper.WaitTimeoutSeconds) { [int]$parsed.wrapper.WaitTimeoutSeconds } else { 3600 }
$LightSlots = if ($parsed.wrapper.LightSlots) { [int]$parsed.wrapper.LightSlots } else { 2 }
$Force = [bool]$parsed.wrapper.Force
$DisableSccache = [bool]$parsed.wrapper.DisableSccache
$SkipCheapGates = [bool]$parsed.wrapper.SkipCheapGates
$CargoArgs = @($parsed.cargo)
if (-not $CargoArgs.Count) { throw "Usage: validate-rust.ps1 [wrapper-options] -- <cargo-args...>" }
Import-Module (Join-Path $Modules "Validation.Scheduler.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Cargo.Network.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Receipt.psm1") -Force -DisableNameChecking
# Re-import public leaves after composite modules so PowerShell 5.1 does not
# hide their exported commands when a dependency is loaded with -Force.
Import-Module (Join-Path $Modules "Validation.Fingerprint.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Evidence.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $RepoRoot "scripts\rust-cache\RustCache.Paths.psm1") -Force -DisableNameChecking

if (-not $SkipCheapGates) {
    Write-Host "VALIDATION_GATE=git_diff_check"
    & git -C $RepoRoot diff --check
    if ($LASTEXITCODE -ne 0) { throw "Cheap gate failed: git diff --check" }
    & git -C $RepoRoot diff --cached --check
    if ($LASTEXITCODE -ne 0) { throw "Cheap gate failed: staged git diff --check" }
    Write-Host "VALIDATION_GATE=source_size"
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $RepoRoot "scripts\check-source-size.ps1")
    if ($LASTEXITCODE -ne 0) { throw "Cheap gate failed: source size" }
    Write-Host "VALIDATION_GATE=rust_format"
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $RepoRoot "scripts\format-rust.ps1")
    if ($LASTEXITCODE -ne 0) { throw "Cheap gate failed: Rust formatting" }
}

$root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
$stateRoot = Join-Path $root "validation-v1"
$CargoArgs = Add-CargoArgumentOnce $CargoArgs '--locked'
$bootstrapLease = $null
try {
    $bootstrapLease = Enter-ValidationLock -LockPath (Join-Path $stateRoot '.cargo-lock-bootstrap.lock') -Kind 'cargo-lock-bootstrap' -TimeoutSeconds $WaitTimeoutSeconds
    Initialize-ValidationCargoLock -RepoRoot $RepoRoot -CargoArgs $CargoArgs
} finally {
    Exit-ValidationLock -Lease $bootstrapLease
}
$fingerprint = Get-ValidationFingerprint -RepoRoot $RepoRoot -CargoArgs $CargoArgs -Domain $Domain -TargetDir $TargetDir -ExecutionOptions ([ordered]@{ disable_sccache=[bool]$DisableSccache; light_slots=$LightSlots })
$resultDir = Join-Path $stateRoot ("evidence\" + $fingerprint.fingerprint)
$summaryPath = Join-Path $resultDir "summary.json"
$prior = if (Test-Path -LiteralPath $summaryPath) { try { Get-Content -Raw -LiteralPath $summaryPath | ConvertFrom-Json } catch { $null } } else { $null }
if (-not $Force -and $prior -and $prior.status -eq "success") {
    $receiptPath = Write-ValidationReceipt -StateRoot $stateRoot -FingerprintDetails $fingerprint -EvidenceSummaryPath $summaryPath -NetworkReportPath $prior.network_report
    Write-Host "VALIDATION_REUSED=true"
    Write-Host "VALIDATION_FINGERPRINT=$($fingerprint.fingerprint)"
    Write-Host "VALIDATION_EVIDENCE=$summaryPath"
    Write-Host "VALIDATION_RECEIPT=$receiptPath"
    exit 0
}

$runLease = $null; $resourceLease = $null
try {
    $runLease = Enter-ValidationLock -LockPath (Join-Path $resultDir ".run.lock") -Kind "fingerprint" -TimeoutSeconds $WaitTimeoutSeconds -PersistWaiter
    $prior = if (Test-Path -LiteralPath $summaryPath) { try { Get-Content -Raw -LiteralPath $summaryPath | ConvertFrom-Json } catch { $null } } else { $null }
    if (-not $Force -and $prior -and $prior.status -eq "success") {
        $receiptPath = Write-ValidationReceipt -StateRoot $stateRoot -FingerprintDetails $fingerprint -EvidenceSummaryPath $summaryPath -NetworkReportPath $prior.network_report
        Write-Host "VALIDATION_REUSED=coalesced_wait"
        Write-Host "VALIDATION_FINGERPRINT=$($fingerprint.fingerprint)"
        Write-Host "VALIDATION_EVIDENCE=$summaryPath"
        Write-Host "VALIDATION_RECEIPT=$receiptPath"
        exit 0
    }
    $class = Get-ValidationResourceClass -CargoArgs $CargoArgs
    $resourceLease = Enter-ValidationResource -StateRoot $stateRoot -Class $class -LightSlots $LightSlots -TimeoutSeconds $WaitTimeoutSeconds
    Write-Host "VALIDATION_OWNER_PID=$PID"
    Write-Host "VALIDATION_OWNER_LEASE=$($runLease.owner.lease_id)"
    Write-Host "VALIDATION_RESOURCE=$class"
    Write-Host "VALIDATION_QUEUE_WAIT_MS=$($resourceLease.wait_ms)"
    $cargoNetwork = Join-Path $RepoRoot "scripts\cargo-network.ps1"
    $networkRoot = Join-Path $resultDir 'network'
    $args = @("-NoProfile","-ExecutionPolicy","Bypass","-File",$cargoNetwork,"-Domain",$Domain,"-ReportRoot",$networkRoot,"-CompileTimeoutSeconds",$WaitTimeoutSeconds)
    if ($DisableSccache) { $args += "-DisableSccache" }
    if ($CacheRoot) { $args += @("-CacheRoot",$CacheRoot) }
    if ($TargetDir) { $args += @("-TargetDir",$TargetDir) }
    $args += '--'
    $args += $CargoArgs
    $startedAt = [DateTime]::UtcNow.ToString("o")
    Write-ValidationJsonAtomic -Path $summaryPath -Value ([ordered]@{
        schema="elon.validation.evidence.v1"; fingerprint=$fingerprint.fingerprint; status="running"
        owner_pid=$PID; owner_lease=$runLease.owner.lease_id; owner_process_start_id=$runLease.owner.process_start_id; resource_class=$class; queue_wait_ms=$resourceLease.wait_ms; coalesced_waiters_path=(Join-Path $resultDir '.run.lock.waiters'); command=@($CargoArgs)
        fingerprint_inputs=$fingerprint.payload; started_utc=$startedAt
        stdout_path=(Join-Path $resultDir "stdout.log"); stderr_path=(Join-Path $resultDir "stderr.log")
    })
    try {
        $result = Invoke-ValidationCapturedProcess -FilePath "powershell" -ArgumentList $args -WorkingDirectory $RepoRoot -EvidenceDirectory $resultDir -TimeoutSeconds $WaitTimeoutSeconds
    } catch {
        $failedAt = [DateTime]::UtcNow.ToString("o")
        Write-ValidationJsonAtomic -Path $summaryPath -Value ([ordered]@{
            schema="elon.validation.evidence.v1"; fingerprint=$fingerprint.fingerprint; status="failed"; exit_code=1
            resource_class=$class; owner_pid=$PID; owner_lease=$runLease.owner.lease_id; owner_process_start_id=$runLease.owner.process_start_id
            queue_wait_ms=$resourceLease.wait_ms; command=@($CargoArgs); fingerprint_inputs=$fingerprint.payload
            started_utc=$startedAt; finished_utc=$failedAt; duration_ms=[int](([DateTime]::Parse($failedAt)-[DateTime]::Parse($startedAt)).TotalMilliseconds); timed_out=$false
            stdout_path=(Join-Path $resultDir "stdout.log"); stderr_path=(Join-Path $resultDir "stderr.log")
            stdout_lines=0; stderr_lines=0; failures=@($_.Exception.Message); tail=@($_.Exception.Message)
        })
        Write-Host "VALIDATION_FINGERPRINT=$($fingerprint.fingerprint)"
        Write-Host "VALIDATION_EVIDENCE=$summaryPath"
        throw
    }
    $summary = [ordered]@{
        schema="elon.validation.evidence.v1"; fingerprint=$fingerprint.fingerprint
        status=if ($result.exit_code -eq 0) { "success" } else { "failed" }
        exit_code=$result.exit_code; resource_class=$class; owner_pid=$PID; owner_lease=$runLease.owner.lease_id; owner_process_start_id=$runLease.owner.process_start_id
        queue_wait_ms=$resourceLease.wait_ms; command=@($CargoArgs); fingerprint_inputs=$fingerprint.payload
        started_utc=$result.started_utc; finished_utc=$result.finished_utc; duration_ms=$result.duration_ms
        stdout_path=$result.stdout_path; stderr_path=$result.stderr_path
        stdout_lines=$result.stdout_lines; stderr_lines=$result.stderr_lines
        failures=$result.failures; tail=$result.tail; timed_out=[bool]$result.timed_out
        network_report=(Join-Path $networkRoot 'cargo-network-report.json')
    }
    Write-ValidationJsonAtomic -Path $summaryPath -Value $summary
    Write-Host "VALIDATION_REUSED=false"
    Write-Host "VALIDATION_FINGERPRINT=$($fingerprint.fingerprint)"
    Write-Host "VALIDATION_EVIDENCE=$summaryPath"
    foreach ($line in $result.tail) { Write-Host $line }
    if ($result.exit_code -ne 0) { exit $result.exit_code }
    $receiptPath = Write-ValidationReceipt -StateRoot $stateRoot -FingerprintDetails $fingerprint -EvidenceSummaryPath $summaryPath -NetworkReportPath $summary.network_report
    Write-Host "VALIDATION_RECEIPT=$receiptPath"
} finally {
    Exit-ValidationResource -Lease $resourceLease
    Exit-ValidationLock -Lease $runLease
    Remove-ExpiredValidationEvidence -EvidenceRoot (Join-Path $stateRoot "evidence")
}
exit 0

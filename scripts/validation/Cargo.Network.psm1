Import-Module "$PSScriptRoot\Cargo.Diagnostics.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\Cargo.SourcePolicy.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\Validation.Evidence.psm1" -Force -DisableNameChecking

function Add-CargoArgumentOnce {
    param([string[]]$Arguments,[Parameter(Mandatory)][string]$Value)
    if ($Arguments -contains $Value) { return @($Arguments) }
    return @($Arguments) + @($Value)
}

function Get-CargoManifestPathFromArguments {
    param([Parameter(Mandatory)][string]$RepoRoot,[string[]]$Arguments)
    $value = $null
    for ($i=0; $i -lt $Arguments.Count; $i++) {
        if ($Arguments[$i] -eq '--manifest-path' -and $i+1 -lt $Arguments.Count) { $value=$Arguments[$i+1]; break }
        if ($Arguments[$i] -like '--manifest-path=*') { $value=$Arguments[$i].Substring('--manifest-path='.Length); break }
    }
    if (-not $value) { $value='Cargo.toml' }
    if([IO.Path]::IsPathRooted($value)){return [IO.Path]::GetFullPath($value)}
    return [IO.Path]::GetFullPath((Join-Path $RepoRoot $value))
}

function Get-CargoFetchArguments {
    param([Parameter(Mandatory)][string]$RepoRoot,[string[]]$ValidationArguments)
    $manifest = Get-CargoManifestPathFromArguments $RepoRoot $ValidationArguments
    $result = @('fetch','--locked','--manifest-path',$manifest)
    for($i=0;$i -lt $ValidationArguments.Count;$i++) {
        $arg=[string]$ValidationArguments[$i]
        if($arg -eq '--target' -and $i+1 -lt $ValidationArguments.Count){$result += @('--target',[string]$ValidationArguments[++$i])}
        elseif($arg -like '--target=*'){$result += $arg}
    }
    return $result
}

function Invoke-CargoManagedAttempt {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$CargoDevPath,
        [Parameter(Mandatory)][string]$EvidenceDirectory,
        [Parameter(Mandatory)][string[]]$CargoArguments,
        [string]$CacheRoot,
        [string]$Domain='agent-validation',
        [string]$TargetDir,
        [string]$CargoHome,
        [int]$TimeoutSeconds=3600,
        [switch]$DisableSccache
    )
    $arguments=@('-NoProfile','-ExecutionPolicy','Bypass','-File',$CargoDevPath,'-BypassValidationOrchestrator','-SkipCacheGc','-Domain',$Domain)
    if($CacheRoot){$arguments += @('-CacheRoot',$CacheRoot)}
    if($TargetDir){$arguments += @('-TargetDir',$TargetDir)}
    if($DisableSccache){$arguments += '-DisableSccache'}
    $arguments += '--'; $arguments += @($CargoArguments)
    $priorCargoHome=[Environment]::GetEnvironmentVariable('CARGO_HOME','Process')
    try {
        if($CargoHome){$env:CARGO_HOME=$CargoHome}
        $result=Invoke-ValidationCapturedProcess -FilePath 'powershell' -ArgumentList $arguments -WorkingDirectory $RepoRoot -EvidenceDirectory $EvidenceDirectory -TimeoutSeconds $TimeoutSeconds
    } finally {
        if($null -eq $priorCargoHome){Remove-Item Env:CARGO_HOME -ErrorAction SilentlyContinue}else{$env:CARGO_HOME=$priorCargoHome}
    }
    $lines=@()
    if(Test-Path $result.stdout_path){$lines += @(Get-Content -LiteralPath $result.stdout_path -ErrorAction SilentlyContinue)}
    if(Test-Path $result.stderr_path){$lines += @(Get-Content -LiteralPath $result.stderr_path -ErrorAction SilentlyContinue)}
    $result | Add-Member -NotePropertyName diagnostic -NotePropertyValue (Get-CargoFailureDiagnostic $lines 'cargo' $result.exit_code -TimedOut:$result.timed_out)
    return $result
}

function Write-CargoNetworkReport {
    param([Parameter(Mandatory)][string]$Path,[Parameter(Mandatory)]$Value)
    New-Item -ItemType Directory -Force -Path (Split-Path $Path -Parent) | Out-Null
    $temporary="$Path.$PID.tmp"; $Value|ConvertTo-Json -Depth 12|Set-Content -LiteralPath $temporary -Encoding UTF8; Move-Item -LiteralPath $temporary -Destination $Path -Force
    return $Path
}

function New-CargoAttemptRecord {
    param([string]$SourceId,[string]$Stage,$Diagnostic,[int]$ExitCode,[int]$DurationMs,[string]$EvidencePath,[bool]$Cached=$false)
    [pscustomobject]@{source_id=$SourceId;stage=$Stage;code=[string]$Diagnostic.code;retryable=[bool]$Diagnostic.retryable;exit_code=$ExitCode;duration_ms=$DurationMs;evidence_path=$EvidencePath;health_cached=$Cached}
}

function Invoke-CargoNetworkValidation {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$CargoDevPath,
        [Parameter(Mandatory)][string]$ReportRoot,
        [Parameter(Mandatory)][string[]]$CargoArguments,
        [Parameter(Mandatory)][string]$ResolvedCacheRoot,
        [string]$PolicyPath=(Join-Path $PSScriptRoot 'cargo-sources.json'),
        [string]$Domain='agent-validation',
        [string]$TargetDir,
        [int]$CompileTimeoutSeconds=3600,
        [switch]$DisableSccache,
        [switch]$SkipOfflineFirst,
        [object[]]$OverrideSources,
        [scriptblock]$HttpInvoker
    )
    New-Item -ItemType Directory -Force -Path $ReportRoot | Out-Null
    $policy=Get-CargoSourcePolicy $PolicyPath
    $sources=if($null -ne $OverrideSources){@($OverrideSources)}else{@($policy.sources)}
    $manifest=Get-CargoManifestPathFromArguments $RepoRoot $CargoArguments
    $lockPath=Join-Path (Split-Path $manifest -Parent) 'Cargo.lock'
    if(-not(Test-Path -LiteralPath $lockPath)){throw "Tracked Cargo.lock is required before managed validation: $lockPath"}
    $locked=Add-CargoArgumentOnce @($CargoArguments) '--locked'
    $attempts=New-Object System.Collections.Generic.List[object]
    $started=[DateTime]::UtcNow; $clock=[Diagnostics.Stopwatch]::StartNew()
    $reportPath=Join-Path $ReportRoot 'cargo-network-report.json'

    if(-not $SkipOfflineFirst){
        $offline=Add-CargoArgumentOnce $locked '--offline'
        $result=Invoke-CargoManagedAttempt $RepoRoot $CargoDevPath (Join-Path $ReportRoot 'offline') $offline $ResolvedCacheRoot $Domain $TargetDir -TimeoutSeconds $CompileTimeoutSeconds -DisableSccache:$DisableSccache
        $attempts.Add((New-CargoAttemptRecord 'local-cache' 'locked_offline_check' $result.diagnostic $result.exit_code $result.duration_ms $result.stderr_path))
        Write-CargoMachineStatus $result.diagnostic
        if($result.exit_code -eq 0){
            $report=[ordered]@{schema='elon.cargo_network_report.v1';status='success';strategy='locked_offline';started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');attempts=$attempts.ToArray();selected_source='local-cache'}
            Write-CargoNetworkReport $reportPath $report|Out-Null; Write-Host "CARGO_NETWORK_REPORT=$reportPath"; return [pscustomobject]@{exit_code=0;status='success';source='local-cache';report_path=$reportPath;attempts=$attempts.ToArray()}
        }
        if($result.diagnostic.code -ne 'CARGO_OFFLINE_MISSING'){
            $report=[ordered]@{schema='elon.cargo_network_report.v1';status='failed';strategy='locked_offline';started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');attempts=$attempts.ToArray();final_diagnostic=$result.diagnostic}
            Write-CargoNetworkReport $reportPath $report|Out-Null; Write-Host "CARGO_NETWORK_REPORT=$reportPath"; return [pscustomobject]@{exit_code=$result.exit_code;status='failed';source='local-cache';report_path=$reportPath;diagnostic=$result.diagnostic;attempts=$attempts.ToArray()}
        }
    }

    $healthPath=Join-Path $ResolvedCacheRoot 'validation-v1\source-health.json'
    $health=Get-CargoSourceHealthState $healthPath
    foreach($source in $sources){
        $remaining=[int]$policy.total_budget_seconds-[int]$clock.Elapsed.TotalSeconds
        if($remaining -le 0){$attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'budget' (New-CargoDiagnostic 'CARGO_NETWORK_BUDGET_EXHAUSTED' 'budget' $false 'Total trusted-source budget exhausted.') 124 ([int]$clock.ElapsedMilliseconds) $null));break}
        if(Test-CargoSourceCircuitOpen $health ([string]$source.id)){$attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'circuit' (New-CargoDiagnostic 'CARGO_SOURCE_CIRCUIT_OPEN' 'circuit' $true 'Source skipped until its circuit cools down.') 75 0 $healthPath));continue}
        $fresh=Test-CargoSourceHealthFresh $health ([string]$source.id) ([int]$policy.health_success_ttl_seconds)
        if(-not $fresh){
            $probeWatch=[Diagnostics.Stopwatch]::StartNew()
            $probe=Test-CargoSourceEndpoint $source $lockPath ([int]$policy.probe_timeout_seconds) ([int]$policy.maximum_redirects) -HttpInvoker $HttpInvoker
            $probeWatch.Stop(); $diagnostic=Get-CargoProbeDiagnostic $probe
            $probeExitCode = if($probe.ok){0}else{69}
            $attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'endpoint_probe' $diagnostic $probeExitCode ([int]$probeWatch.ElapsedMilliseconds) $null))
            if(-not $probe.ok){Update-CargoSourceHealthState $healthPath ([string]$source.id) $false $diagnostic.code ([int]$policy.circuit_failure_threshold) ([int]$policy.circuit_open_seconds)|Out-Null;$health=Get-CargoSourceHealthState $healthPath;continue}
        }
        $remaining=[int]$policy.total_budget_seconds-[int]$clock.Elapsed.TotalSeconds
        if($remaining -le 0){$attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'budget' (New-CargoDiagnostic 'CARGO_NETWORK_BUDGET_EXHAUSTED' 'budget' $false 'Total trusted-source budget exhausted after probing.') 124 ([int]$clock.ElapsedMilliseconds) $null));break}
        $sourceHome=Join-Path $ResolvedCacheRoot ("cargo-home\"+[string]$source.id)
        New-CargoSourceHomeConfig $source $sourceHome ([int]$policy.fetch_timeout_seconds)|Out-Null
        $fetchTimeout=[Math]::Max(1,[Math]::Min([int]$policy.fetch_timeout_seconds,$remaining))
        $fetch=Invoke-CargoManagedAttempt $RepoRoot $CargoDevPath (Join-Path $ReportRoot ("fetch-"+$source.id)) (Get-CargoFetchArguments $RepoRoot $locked) $ResolvedCacheRoot $Domain $TargetDir $sourceHome $fetchTimeout -DisableSccache
        $attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'locked_fetch' $fetch.diagnostic $fetch.exit_code $fetch.duration_ms $fetch.stderr_path $fresh))
        if($fetch.exit_code -ne 0){Update-CargoSourceHealthState $healthPath ([string]$source.id) $false $fetch.diagnostic.code ([int]$policy.circuit_failure_threshold) ([int]$policy.circuit_open_seconds)|Out-Null;$health=Get-CargoSourceHealthState $healthPath;if(-not $fetch.diagnostic.retryable){break};continue}
        Update-CargoSourceHealthState $healthPath ([string]$source.id) $true 'CARGO_SOURCE_HEALTHY' ([int]$policy.circuit_failure_threshold) ([int]$policy.circuit_open_seconds)|Out-Null
        $check=Invoke-CargoManagedAttempt $RepoRoot $CargoDevPath (Join-Path $ReportRoot ("check-"+$source.id)) (Add-CargoArgumentOnce $locked '--offline') $ResolvedCacheRoot $Domain $TargetDir $sourceHome $CompileTimeoutSeconds -DisableSccache:$DisableSccache
        $attempts.Add((New-CargoAttemptRecord ([string]$source.id) 'locked_offline_check_after_fetch' $check.diagnostic $check.exit_code $check.duration_ms $check.stderr_path))
        Write-CargoMachineStatus $check.diagnostic
        if($check.exit_code -eq 0){$report=[ordered]@{schema='elon.cargo_network_report.v1';status='success';strategy='trusted_failover';started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');attempts=$attempts.ToArray();selected_source=[string]$source.id};Write-CargoNetworkReport $reportPath $report|Out-Null;Write-Host "CARGO_SOURCE_SELECTED=$($source.id)";Write-Host "CARGO_NETWORK_REPORT=$reportPath";return [pscustomobject]@{exit_code=0;status='success';source=[string]$source.id;report_path=$reportPath;attempts=$attempts.ToArray()}
        }
        if($check.diagnostic.code -ne 'CARGO_OFFLINE_MISSING'){$report=[ordered]@{schema='elon.cargo_network_report.v1';status='failed';strategy='trusted_failover';started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');attempts=$attempts.ToArray();final_diagnostic=$check.diagnostic};Write-CargoNetworkReport $reportPath $report|Out-Null;return [pscustomobject]@{exit_code=$check.exit_code;status='failed';source=[string]$source.id;report_path=$reportPath;diagnostic=$check.diagnostic;attempts=$attempts.ToArray()}}
    }
    $repair=[ordered]@{schema='elon.cargo_source_repair.v1';status='CARGO_SOURCE_REPAIR_REQUIRED';generated_utc=[DateTime]::UtcNow.ToString('o');trusted_policy=$PolicyPath;attempts=$attempts.ToArray();ai_handoff=[ordered]@{protocol='elon.ai.cargo_source_repair.v1';allowed_actions=@('search_official_or_operator_pages','test_https_candidate_temporarily','continue_with_isolated_locked_cache');forbidden_actions=@('trust_random_web_mirror','write_user_global_cargo_config','send_credentials');candidate_requirements=@('https','bounded_redirects','valid_config_json','https_download_endpoint','operator_evidence','cargo_lock_checksum','isolated_locked_fetch_and_check');permanent_addition='commit cargo-sources.json plus offline fault-injection tests'};repair_command='powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-source-repair.ps1 -Index sparse+https://HOST/PATH/ -Evidence https://OPERATOR/OFFICIAL-DOC'}
    $repairPath=Join-Path $ReportRoot 'cargo-source-repair-required.json';Write-CargoNetworkReport $repairPath $repair|Out-Null
    $diagnostic=New-CargoDiagnostic 'CARGO_SOURCE_REPAIR_REQUIRED' 'trusted_failover' $false 'All trusted Cargo sources failed; AI candidate repair protocol is required.' @($repairPath)
    Write-CargoMachineStatus $diagnostic;Write-Host 'AI_TAKEOVER_REQUIRED=true';Write-Host 'AI_TAKEOVER_PROTOCOL=elon.ai.cargo_source_repair.v1';Write-Host "CARGO_SOURCE_REPAIR_REPORT=$repairPath"
    return [pscustomobject]@{exit_code=86;status='repair_required';source=$null;report_path=$repairPath;diagnostic=$diagnostic;attempts=$attempts.ToArray()}
}

Export-ModuleMember -Function Add-CargoArgumentOnce, Get-CargoManifestPathFromArguments, Get-CargoFetchArguments, Invoke-CargoManagedAttempt, Write-CargoNetworkReport, Invoke-CargoNetworkValidation

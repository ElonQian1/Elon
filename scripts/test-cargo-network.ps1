$ErrorActionPreference='Stop'
$repoRoot=(& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$modules=Join-Path $PSScriptRoot 'validation'
Import-Module (Join-Path $modules 'Cargo.Network.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Validation.Receipt.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Cargo.SourcePolicy.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Cargo.Diagnostics.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Validation.Fingerprint.psm1') -Force -DisableNameChecking
$script:assertions=0
function Assert-True([bool]$Value,[string]$Message){$script:assertions++;if(-not $Value){throw "ASSERT FAILED: $Message"}}
function Assert-Equal($Expected,$Actual,[string]$Message){$script:assertions++;if($Expected -ne $Actual){throw "ASSERT FAILED: $Message expected=$Expected actual=$Actual"}}
$temp=Join-Path $env:TEMP ('elon-cargo-network-test-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $temp|Out-Null
try{
    $policy=Get-CargoSourcePolicy
    Assert-Equal 3 @($policy.sources).Count 'initial trust set count'
    Assert-Equal 'crates-io-official' $policy.sources[0].id 'official source priority'
    $nonHttpsRejected=$false;try{Assert-CargoSparseHttpsUri 'sparse+http://mirror.invalid/'|Out-Null}catch{$nonHttpsRejected=$true}
    Assert-True $nonHttpsRejected 'non-HTTPS sparse source must be rejected'
    Assert-True (-not(Test-CargoRedirectTarget ([Uri]'http://safe.example.test/') @('example.test'))) 'HTTPS downgrade redirect must be rejected'
    Assert-True (-not(Test-CargoRedirectTarget ([Uri]'https://evil.invalid/') @('example.test'))) 'cross-operator redirect must be rejected'
    Assert-True (Test-CargoRedirectTarget ([Uri]'https://cdn.example.test/path') @('example.test')) 'operator subdomain redirect should pass'

    $lock=Join-Path $temp 'Cargo.lock'
    @'
version = 4
[[package]]
name = "alpha"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
'@|Set-Content -LiteralPath $lock -Encoding UTF8
    $source=[pscustomobject]@{id='test';index='sparse+https://index.example.test/';evidence='https://example.test/docs';operator_domains=@('example.test')}
    $okInvoker={param($uri,$timeout,$redirects,$domains,$rangeOnly);if($uri.AbsolutePath -like '*/config.json'){[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=200;body='{"dl":"https://downloads.example.test/api/v1/crates"}'}}else{[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=206;body=''}}}
    $probe=Test-CargoSourceEndpoint $source $lock 2 1 -HttpInvoker $okInvoker
    Assert-True $probe.ok 'valid config and lock-derived download endpoint'
    Assert-True ($probe.download_uri -like '*alpha/1.2.3/download') 'download endpoint must derive from Cargo.lock package'
    $invalidJson={param($uri);[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=200;body='{bad'}}
    Assert-Equal 'CARGO_SOURCE_INVALID_CONFIG_JSON' (Test-CargoSourceEndpoint $source $lock 2 1 -HttpInvoker $invalidJson).code 'invalid config JSON classification'
    $notFound={param($uri);[pscustomobject]@{ok=$false;code='CARGO_SOURCE_HTTP_STATUS';message='HTTP 404';uri=$uri.AbsoluteUri;status_code=404;body=''}}
    Assert-Equal 'CARGO_INDEX_FAILURE' (Test-CargoSourceEndpoint $source $lock 2 1 -HttpInvoker $notFound).code 'index 404 classification'
    $download404={param($uri);if($uri.AbsolutePath -like '*/config.json'){[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=200;body='{"dl":"https://downloads.example.test/api"}'}}else{[pscustomobject]@{ok=$false;code='CARGO_SOURCE_HTTP_STATUS';message='HTTP 404';uri=$uri.AbsoluteUri;status_code=404;body=''}}}
    Assert-Equal 'CARGO_CRATE_DOWNLOAD_FAILURE' (Test-CargoSourceEndpoint $source $lock 2 1 -HttpInvoker $download404).code 'crate download 404 classification'
    $slow={param($uri);[pscustomobject]@{ok=$false;code='CARGO_SOURCE_TIMEOUT';message='bounded timeout';uri=$uri.AbsoluteUri;status_code=$null;body=''}}
    $slowProbe=Test-CargoSourceEndpoint $source $lock 1 1 -HttpInvoker $slow
    Assert-Equal 'CARGO_INDEX_FAILURE' $slowProbe.code 'slow index stage classification'
    Assert-Equal 'CARGO_SOURCE_TIMEOUT' $slowProbe.cause_code 'slow source cause classification'
    $httpDownload={param($uri);[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=200;body='{"dl":"http://downloads.example.test/api"}'}}
    Assert-Equal 'CARGO_SOURCE_POLICY_REJECTED' (Test-CargoSourceEndpoint $source $lock 2 1 -HttpInvoker $httpDownload).code 'non-HTTPS download endpoint rejection'
    $evidenceInvoker={param($uri);[pscustomobject]@{ok=$true;code='OK';message='ok';uri=$uri.AbsoluteUri;status_code=200;body='Use sparse+https://index.example.test/ for Cargo.'}}
    Assert-True (Test-CargoCandidateEvidence 'sparse+https://index.example.test/' 'https://example.test/cargo' 2 1 $evidenceInvoker).ok 'operator parent-domain evidence should pass'
    Assert-Equal 'CARGO_CANDIDATE_EVIDENCE_REJECTED' (Test-CargoCandidateEvidence 'sparse+https://index.example.test/' 'https://random.invalid/post' 2 1 $evidenceInvoker).code 'random web evidence must be rejected'

    $cases=@{
        'dns'=@('Could not resolve host: index.crates.io','CARGO_DNS_FAILURE',$true)
        'tls'=@('SSL certificate problem','CARGO_TLS_FAILURE',$true)
        'proxy'=@('407 Proxy Authentication Required','CARGO_PROXY_FAILURE',$true)
        'git'=@('failed to clone into git dependency','CARGO_GIT_DEPENDENCY_FAILURE',$false)
        'crate'=@('failed to download crate serde','CARGO_CRATE_DOWNLOAD_FAILURE',$true)
        'index'=@('failed to query replaced source registry','CARGO_INDEX_FAILURE',$true)
        'offline'=@('attempting to make an HTTP request, but --offline was specified','CARGO_OFFLINE_MISSING',$true)
        'test'=@('proxy configuration was discovered`ntest result: FAILED. 1 passed; 1 failed','CARGO_TEST_FAILURE',$false)
        'compile'=@('error[E0308]: mismatched types','RUST_COMPILE_ERROR',$false)
        'lock'=@('Blocking waiting for file lock on package cache','CARGO_CACHE_LOCKED',$false)
        'disk'=@('No space left on device (os error 112)','CARGO_DISK_CRITICAL',$false)
    }
    foreach($name in $cases.Keys){$diag=Get-CargoFailureDiagnostic @($cases[$name][0]);Assert-Equal $cases[$name][1] $diag.code "$name diagnostic";Assert-Equal $cases[$name][2] $diag.retryable "$name retry contract"}
    Assert-Equal 'CARGO_UNKNOWN_FAILURE' (Get-CargoFailureDiagnostic @('warning: unused PatchApplyReadinessLevel','error: Unrecognized option: offline')).code 'TLS token must not match inside Rust identifiers'

    $testArguments=@('test','--manifest-path','server\Cargo.toml','node_agent_cli_sidecar','--','--nocapture')
    $offlineArguments=@(Add-CargoArgumentOnce $testArguments '--offline')
    Assert-True ([Array]::IndexOf($offlineArguments,'--offline') -lt [Array]::IndexOf($offlineArguments,'--')) 'Cargo global flags must be inserted before test harness separator'
    Assert-Equal '--nocapture' $offlineArguments[-1] 'test harness arguments must keep their order'

    $healthPath=Join-Path $temp 'health.json'
    Update-CargoSourceHealthState $healthPath 'test' $false 'timeout' 2 300|Out-Null
    Update-CargoSourceHealthState $healthPath 'test' $false 'timeout' 2 300|Out-Null
    Assert-True (Test-CargoSourceCircuitOpen (Get-CargoSourceHealthState $healthPath) 'test') 'two failures must open circuit'
    Update-CargoSourceHealthState $healthPath 'test' $true 'ok' 2 300|Out-Null
    Assert-True (Test-CargoSourceHealthFresh (Get-CargoSourceHealthState $healthPath) 'test' 60) 'success must cache health and close circuit'

    $fake=Join-Path $temp 'fake-cargo-dev.ps1';$callLog=Join-Path $temp 'calls.log'
    @'
$separator=[Array]::IndexOf($args,'--');$cargoArgs=if($separator -ge 0){@($args[($separator+1)..($args.Count-1)])}else{@($args)}
$command=$cargoArgs[0];$cargoHomeValue=[string]$env:CARGO_HOME;$scenario=[string]$env:ELON_TEST_CARGO_SCENARIO
Add-Content -LiteralPath $env:ELON_TEST_CARGO_CALL_LOG -Value ("$scenario|$command|$cargoHomeValue|"+($cargoArgs -join ' '))
if($scenario -eq 'offline_hit'){exit 0}
if($scenario -eq 'compile_error'){Write-Error 'error[E0308]: mismatched types' -ErrorAction Continue;exit 101}
if($scenario -eq 'test_failure'){Write-Output 'test node_agent_failure ... FAILED';Write-Output 'test result: FAILED. 1 passed; 1 failed';Write-Error 'proxy configuration was discovered' -ErrorAction Continue;Write-Error 'error: test failed, to rerun pass `--bin elon-pc-node`' -ErrorAction Continue;exit 101}
if($command -eq 'check' -and [string]::IsNullOrWhiteSpace($cargoHomeValue)){Write-Error 'attempting to make an HTTP request, but --offline was specified' -ErrorAction Continue;exit 101}
if($scenario -eq 'fallback' -and $command -eq 'fetch' -and $cargoHomeValue -like '*source-a'){Write-Error 'Could not resolve host: source-a' -ErrorAction Continue;exit 101}
if($scenario -eq 'all_fetch_fail' -and $command -eq 'fetch'){Write-Error 'failed to update registry index' -ErrorAction Continue;exit 101}
exit 0
'@|Set-Content -LiteralPath $fake -Encoding UTF8
    $cache=Join-Path $temp 'cache';$args=@('check','--manifest-path',(Join-Path $repoRoot 'server\Cargo.toml'),'--workspace','--quiet','--locked')
    $sources=@([pscustomobject]@{id='source-a';index='sparse+https://a.example.test/';evidence='https://a.example.test/docs';operator_domains=@('example.test')},[pscustomobject]@{id='source-b';index='sparse+https://b.example.test/';evidence='https://b.example.test/docs';operator_domains=@('example.test')})
    $env:ELON_TEST_CARGO_CALL_LOG=$callLog
    $env:ELON_TEST_CARGO_SCENARIO='offline_hit';$result=Invoke-CargoNetworkValidation $repoRoot $fake (Join-Path $temp 'offline-hit') $args $cache -DisableSccache -OverrideSources $sources -HttpInvoker $okInvoker
    Assert-Equal 0 $result.exit_code 'offline cache hit';Assert-Equal 1 @(Get-Content $callLog).Count 'offline hit must not probe Cargo sources'
    Clear-Content $callLog;$env:ELON_TEST_CARGO_SCENARIO='compile_error';$result=Invoke-CargoNetworkValidation $repoRoot $fake (Join-Path $temp 'compile') $args $cache -DisableSccache -OverrideSources $sources -HttpInvoker $okInvoker
    Assert-Equal 'RUST_COMPILE_ERROR' $result.diagnostic.code 'compile error classification';Assert-Equal 1 @(Get-Content $callLog).Count 'compile error must not retry sources'
    Clear-Content $callLog;$env:ELON_TEST_CARGO_SCENARIO='test_failure';$captured=@(& { param([string[]]$capturedArguments) Invoke-CargoNetworkValidation $repoRoot $fake (Join-Path $temp 'test-failure') $capturedArguments $cache -DisableSccache -OverrideSources $sources -HttpInvoker $okInvoker } $args 6>&1);$result=@($captured|Where-Object { $_ -is [pscustomobject] -and $_.PSObject.Properties['exit_code'] }|Select-Object -Last 1);$capturedText=@($captured|ForEach-Object {if($_ -is [System.Management.Automation.InformationRecord]){[string]$_.MessageData}else{[string]$_}})-join "`n"
    Assert-Equal 'CARGO_TEST_FAILURE' $result.diagnostic.code 'test failure must not be misclassified as proxy failure';Assert-True ($capturedText -match 'CARGO_ATTEMPT_LOG_BEGIN stage=locked_offline_check stream=stdout') 'stdout evidence must remain separate from stderr';Assert-True ($capturedText -match 'CARGO_ATTEMPT_LOG .*test result: FAILED') 'test failure tail must reach outer validation logs'
    Clear-Content $callLog;$env:ELON_TEST_CARGO_SCENARIO='fallback';$result=Invoke-CargoNetworkValidation $repoRoot $fake (Join-Path $temp 'fallback') $args (Join-Path $temp 'fallback-cache') -DisableSccache -OverrideSources $sources -HttpInvoker $okInvoker
    Assert-Equal 0 $result.exit_code 'backup source success';Assert-Equal 'source-b' $result.source 'deterministic failover order';Assert-True (@(Get-Content $callLog).Count -eq 4) 'offline, failed fetch, successful fetch, offline check'
    Clear-Content $callLog;$env:ELON_TEST_CARGO_SCENARIO='all_fetch_fail';$result=Invoke-CargoNetworkValidation $repoRoot $fake (Join-Path $temp 'all-fail') $args (Join-Path $temp 'all-fail-cache') -DisableSccache -OverrideSources $sources -HttpInvoker $okInvoker
    Assert-Equal 86 $result.exit_code 'all trusted sources require repair';$repair=Get-Content -Raw $result.report_path|ConvertFrom-Json;Assert-Equal 'CARGO_SOURCE_REPAIR_REQUIRED' $repair.status 'repair report status';Assert-Equal 'elon.ai.cargo_source_repair.v1' $repair.ai_handoff.protocol 'AI handoff protocol'

    $state=Join-Path $temp 'receipt-state';$details=Get-ValidationFingerprint $repoRoot $args 'agent-validation' $null ([ordered]@{disable_sccache=$true;light_slots=2})
    $summary=Join-Path $temp 'summary.json';[ordered]@{schema='elon.validation.evidence.v1';status='success';fingerprint=$details.fingerprint}|ConvertTo-Json|Set-Content $summary
    $receiptPath=Write-ValidationReceipt $state $details $summary
    Assert-True (Test-ValidationReceipt $state $details).valid 'receipt hit'
    $receipt=Get-Content -Raw $receiptPath|ConvertFrom-Json;$receipt.fingerprint='invalid';$receipt|ConvertTo-Json -Depth 8|Set-Content $receiptPath
    Assert-Equal 'fingerprint_changed' (Test-ValidationReceipt $state $details).code 'receipt invalidation'
}finally{
    Remove-Item Env:ELON_TEST_CARGO_CALL_LOG,Env:ELON_TEST_CARGO_SCENARIO -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "PASS: Cargo network and receipt fault injection ($script:assertions assertions)"

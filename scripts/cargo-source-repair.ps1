[CmdletBinding()]
param([Parameter(Mandatory)][string]$Index,[Parameter(Mandatory)][string]$Evidence,[string]$CacheRoot,[int]$TimeoutSeconds=8)
$ErrorActionPreference='Stop'
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$modules=Join-Path $PSScriptRoot 'validation'
Import-Module (Join-Path $modules 'Cargo.Network.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Cargo.SourcePolicy.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'rust-cache\RustCache.Paths.psm1') -Force -DisableNameChecking
$evidenceResult=Test-CargoCandidateEvidence -Index $Index -Evidence $Evidence -TimeoutSeconds $TimeoutSeconds
Write-Host ('CARGO_CANDIDATE_EVIDENCE_JSON='+($evidenceResult|ConvertTo-Json -Compress))
if(-not $evidenceResult.ok){exit 87}
$sha=[Security.Cryptography.SHA256]::Create();try{$id='candidate-'+(([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Index)))).Replace('-','').Substring(0,12).ToLowerInvariant())}finally{$sha.Dispose()}
$indexUri=Assert-CargoSparseHttpsUri $Index 'candidate index'
$source=[pscustomobject]@{id=$id;index=$Index;evidence=$Evidence;operator_domains=@($indexUri.Host)}
$lockPath=Join-Path $repoRoot 'server\Cargo.lock'
$probe=Test-CargoSourceEndpoint -Source $source -CargoLockPath $lockPath -TimeoutSeconds $TimeoutSeconds
Write-Host ('CARGO_CANDIDATE_PROBE_JSON='+($probe|ConvertTo-Json -Compress))
if(-not $probe.ok){exit 88}
$resolved=Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $repoRoot
$reportRoot=Join-Path $resolved ("validation-v1\candidate-repair\$id-"+[DateTime]::UtcNow.ToString('yyyyMMddHHmmss'))
$cargoArgs=@('check','--manifest-path','server\Cargo.toml','--workspace','--quiet','--locked')
$result=Invoke-CargoNetworkValidation -RepoRoot $repoRoot -CargoDevPath (Join-Path $PSScriptRoot 'cargo-dev.ps1') -ReportRoot $reportRoot -CargoArguments $cargoArgs -ResolvedCacheRoot $resolved -SkipOfflineFirst -OverrideSources @($source)
Write-Host "CARGO_CANDIDATE_TEMPORARY_ONLY=true"
Write-Host "CARGO_CANDIDATE_REPORT=$($result.report_path)"
Write-Host 'CARGO_CANDIDATE_PERMANENT_RULE=commit policy and fault-injection tests before trust'
exit [int]$result.exit_code

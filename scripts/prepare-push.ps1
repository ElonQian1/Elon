[CmdletBinding()]
param([string]$CacheRoot,[switch]$ReceiptOnly,[switch]$DisableSccache,[switch]$SkipCheapGates,[string]$Domain='agent-validation')
$ErrorActionPreference='Stop'
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$modules=Join-Path $PSScriptRoot 'validation'
Import-Module (Join-Path $PSScriptRoot 'rust-cache\RustCache.Policy.psm1') -Force -DisableNameChecking
$requestedDomain=$Domain
$Domain=Resolve-RustCacheDomain -ProjectRoot $repoRoot -Domain $Domain
if($requestedDomain -ne $Domain){Write-Host "RUST_CACHE_DOMAIN_CANONICALIZED=$requestedDomain->$Domain"}
Import-Module (Join-Path $modules 'Validation.Receipt.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Validation.Fingerprint.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'rust-cache\RustCache.Paths.psm1') -Force -DisableNameChecking
$cargoArgs=@('check','--manifest-path','server\Cargo.toml','--workspace','--quiet','--locked')
$resolved=Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $repoRoot
$stateRoot=Join-Path $resolved 'validation-v1'
$details=Get-ValidationFingerprint -RepoRoot $repoRoot -CargoArgs $cargoArgs -Domain $Domain -ExecutionOptions ([ordered]@{disable_sccache=[bool]$DisableSccache;light_slots=2})
$receipt=Test-ValidationReceipt -StateRoot $stateRoot -FingerprintDetails $details
Write-Host "PUSH_RECEIPT_STATUS=$($receipt.code)"
Write-Host "PUSH_RECEIPT_PATH=$($receipt.path)"
if($receipt.valid){exit 0}
if($ReceiptOnly){Write-Error 'A current Rust validation receipt is required.' -ErrorAction Continue;exit 42}
Write-Host 'PUSH_PREPARE_ACTION=full_validation'
$arguments=@('-Domain',$Domain)
if($CacheRoot){$arguments+=@('-CacheRoot',$CacheRoot)}
if($DisableSccache){$arguments+='-DisableSccache'}
if($SkipCheapGates){$arguments+='-SkipCheapGates'}
$arguments+='--';$arguments+=$cargoArgs
& (Join-Path $PSScriptRoot 'validate-rust.ps1') @arguments
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$details=Get-ValidationFingerprint -RepoRoot $repoRoot -CargoArgs $cargoArgs -Domain $Domain -ExecutionOptions ([ordered]@{disable_sccache=[bool]$DisableSccache;light_slots=2})
$receipt=Test-ValidationReceipt -StateRoot $stateRoot -FingerprintDetails $details
if(-not $receipt.valid){Write-Error "Validation completed without a valid receipt: $($receipt.code)" -ErrorAction Continue;exit 43}
Write-Host 'PUSH_RECEIPT_STATUS=prepared'
Write-Host "PUSH_RECEIPT_PATH=$($receipt.path)"
exit 0

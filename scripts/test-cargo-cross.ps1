$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$scriptPath = Join-Path $PSScriptRoot 'cargo-cross.ps1'
$assertions = 0

function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:assertions++
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Invoke-Plan {
    param([string[]]$CargoArgs)
    $output = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -PlanOnly -- @CargoArgs 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "Plan failed: $($output -join [Environment]::NewLine)"
    }
    $line = $output | Where-Object { [string]$_ -like 'RUST_CROSS_PLAN_JSON=*' } | Select-Object -Last 1
    if (-not $line) { throw 'Plan output did not contain RUST_CROSS_PLAN_JSON.' }
    return ([string]$line).Substring('RUST_CROSS_PLAN_JSON='.Length) | ConvertFrom-Json
}

function Assert-Fails {
    param([string[]]$CargoArgs, [string]$Pattern)
    $priorErrorAction = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -PlanOnly -- @CargoArgs 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $priorErrorAction
    }
    Assert-True ($exitCode -ne 0) "expected failure for: $($CargoArgs -join ' ')"
    Assert-True (($output -join "`n") -match $Pattern) "failure should mention $Pattern"
}

$target = 'x86_64-unknown-linux-musl'
$cargoArgs = @('zigbuild', '--target', $target, '--manifest-path', 'server\Cargo.toml', '--locked', '--tests')
$plan = Invoke-Plan -CargoArgs $cargoArgs
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$expectedTargetRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot '.ai-tmp\cargo-cross-target'))

Assert-True ($plan.schema -eq 'elon.rust_cross_cache_plan.v1') 'schema'
Assert-True ($plan.target -eq $target) 'target triple'
Assert-True ($plan.cache_domain -eq 'agent-validation') 'stable cache domain'
Assert-True ($plan.shared_build_partition -eq "cross-$target") 'stable shared partition'
Assert-True ($plan.target_dir.StartsWith($expectedTargetRoot + [IO.Path]::DirectorySeparatorChar)) 'target must stay under worktree .ai-tmp'
Assert-True ($plan.target_lifecycle -eq 'task_temporary') 'target lifecycle'
Assert-True ($plan.intermediate_lifecycle -eq 'rust_cache_v2_managed') 'intermediate lifecycle'
Assert-True ((@($plan.cargo_args) -join "`n") -eq ($cargoArgs -join "`n")) 'Cargo arguments remain exact'

$equalsPlan = Invoke-Plan -CargoArgs @('zigbuild', "--target=$target", '--manifest-path', 'server\Cargo.toml')
Assert-True ($equalsPlan.shared_build_partition -eq $plan.shared_build_partition) 'equivalent target syntax reuses partition'
Assert-True ($equalsPlan.target_dir -eq $plan.target_dir) 'equivalent target syntax reuses task target path'

Assert-Fails -CargoArgs @('zigbuild', '--manifest-path', 'server\Cargo.toml') -Pattern 'require.*--target'
Assert-Fails -CargoArgs @('zigbuild', '--target', $target, '--target', 'aarch64-unknown-linux-musl') -Pattern 'exactly one target'
Assert-Fails -CargoArgs @('zigbuild', '--target', '.\custom-target.json') -Pattern 'standard target triple'

$source = Get-Content -LiteralPath $scriptPath -Raw
Assert-True ($source -match 'Invoke-RustCacheCargo') 'wrapper delegates to the managed Rust cache runtime'
Assert-True ($source -match '-SharedBuildPartition \$sharedBuildPartition') 'wrapper uses a managed shared partition'
Assert-True ($source -notmatch '\$env:CARGO_TARGET_DIR\s*=') 'wrapper never assigns a raw caller target environment'

$integrationRoot = Join-Path $repoRoot '.ai-tmp\cargo-cross-contract-test'
$fakeBin = Join-Path $integrationRoot 'bin'
$cacheRoot = Join-Path $integrationRoot 'cache'
$capturePath = Join-Path $integrationRoot 'cargo-invocation.json'
New-Item -ItemType Directory -Force -Path $fakeBin | Out-Null
@'
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fake-cargo.ps1" %*
exit /b %ERRORLEVEL%
'@ | Set-Content -LiteralPath (Join-Path $fakeBin 'cargo.cmd') -Encoding ASCII
@'
[ordered]@{
    args = @($args)
    build_dir = $env:CARGO_BUILD_BUILD_DIR
    target_dir = $env:CARGO_TARGET_DIR
    incremental = $env:CARGO_INCREMENTAL
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $env:ELON_CROSS_TEST_CAPTURE -Encoding UTF8
exit 0
'@ | Set-Content -LiteralPath (Join-Path $fakeBin 'fake-cargo.ps1') -Encoding UTF8

$originalPath = $env:PATH
try {
    $env:PATH = "$fakeBin;$originalPath"
    $env:ELON_CROSS_TEST_CAPTURE = $capturePath
    $integrationOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath `
        -CacheRoot $cacheRoot -DisableSccache -SkipCacheGc -- @cargoArgs 2>&1)
    Assert-True ($LASTEXITCODE -eq 0) "managed integration route: $($integrationOutput -join "`n")"
    Assert-True (Test-Path -LiteralPath $capturePath) 'fake Cargo received the invocation'
    $capture = Get-Content -LiteralPath $capturePath -Raw | ConvertFrom-Json
    $managedBuildRoot = [IO.Path]::GetFullPath((Join-Path $cacheRoot 'build'))
    Assert-True ($capture.build_dir.StartsWith($managedBuildRoot + [IO.Path]::DirectorySeparatorChar)) 'build-dir is cache-v2 managed'
    Assert-True ($capture.build_dir -match 'agent-validation\\shared-cross-x86_64-unknown-linux-musl$') 'build-dir uses the stable shared partition'
    Assert-True ($capture.target_dir -eq $plan.target_dir) 'real invocation target matches the plan'
    Assert-True ((@($capture.args) -join "`n") -eq ($cargoArgs -join "`n")) 'real Cargo arguments remain exact'
    Assert-True ($capture.incremental -eq '0') 'agent validation disables incremental accumulation'
} finally {
    $env:PATH = $originalPath
    Remove-Item Env:ELON_CROSS_TEST_CAPTURE -ErrorAction SilentlyContinue
}

Write-Host "PASS: cargo cross cache routing tests ($assertions assertions)." -ForegroundColor Green

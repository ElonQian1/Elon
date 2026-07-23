param([switch]$Force)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$total = [Diagnostics.Stopwatch]::StartNew()

function Invoke-TimedPowerShellGate {
    param([Parameter(Mandatory = $true)][string]$Name, [Parameter(Mandatory = $true)][string]$ScriptPath)
    $timer = [Diagnostics.Stopwatch]::StartNew()
    & powershell -NoProfile -ExecutionPolicy Bypass -File $ScriptPath
    $exit = $LASTEXITCODE
    $timer.Stop()
    Write-Output "NODE_AGENT_VALIDATION_STAGE=$Name"
    Write-Output "NODE_AGENT_VALIDATION_STAGE_DURATION_MS=$($timer.ElapsedMilliseconds)"
    if ($exit -ne 0) { throw "$Name failed with exit code $exit" }
}

Invoke-TimedPowerShellGate -Name 'powershell_local_first' `
    -ScriptPath (Join-Path $PSScriptRoot 'test-node-agent-local-first-release.ps1')
Invoke-TimedPowerShellGate -Name 'powershell_release_contract' `
    -ScriptPath (Join-Path $PSScriptRoot 'test-node-agent-release-contract.ps1')

$cargoTimer = [Diagnostics.Stopwatch]::StartNew()
$validator = Join-Path $PSScriptRoot 'validate-rust.ps1'
$validatorArgs = @('-Domain','node-agent-local-first-v1')
if ($Force) { $validatorArgs += '-Force' }
$validatorArgs += @('--','test','--manifest-path','server\Cargo.toml','--bin','elon-pc-node','node_agent_update_','--','--nocapture')
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    $cargoOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $validator @validatorArgs 2>&1)
    $cargoExit = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousPreference
}
foreach ($line in $cargoOutput) { Write-Output $line }
$cargoTimer.Stop()
$reused = @($cargoOutput | Where-Object { [string]$_ -like 'VALIDATION_REUSED=*' } | Select-Object -Last 1)
$reuseValue = if ($reused.Count -gt 0) { ([string]$reused[0]).Substring('VALIDATION_REUSED='.Length) } else { 'unknown' }
Write-Output 'NODE_AGENT_VALIDATION_CARGO_INVOCATIONS=1'
Write-Output "NODE_AGENT_VALIDATION_CARGO_CACHE_REUSED=$reuseValue"
Write-Output "NODE_AGENT_VALIDATION_CARGO_DURATION_MS=$($cargoTimer.ElapsedMilliseconds)"
if ($cargoExit -ne 0) { throw "Aggregated node-agent Cargo validation failed with exit code $cargoExit" }

$total.Stop()
Write-Output "NODE_AGENT_VALIDATION_TOTAL_DURATION_MS=$($total.ElapsedMilliseconds)"

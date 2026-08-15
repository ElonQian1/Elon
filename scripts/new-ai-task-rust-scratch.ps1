param(
    [Parameter(Mandatory = $true)][string]$TaskWorktree,
    [Parameter(Mandatory = $true)][string]$TaskContract,
    [Parameter(Mandatory = $true)][string]$Purpose,
    [string]$ScratchRoot = ''
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'ai-task-finish-contract.ps1')
. (Join-Path $PSScriptRoot 'ai-task-external-artifacts.ps1')

$scratch = New-AiTaskRustScratch `
    -RepoPath $TaskWorktree `
    -ContractId $TaskContract `
    -Purpose $Purpose `
    -ScratchRoot $ScratchRoot

Write-Output "AI_TASK_RUST_SCRATCH=$($scratch.scratch_path)"
Write-Output "ELON_RUST_CACHE_ROOT=$($scratch.cache_root)"
Write-Output "CARGO_TARGET_DIR=$($scratch.target_dir)"

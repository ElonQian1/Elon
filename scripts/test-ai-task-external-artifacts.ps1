$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>&1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Run this test inside the Git repository.'
}
$repoRoot = $repoRoot.Trim()

function Invoke-Git {
    param([string]$Path, [string[]]$Arguments)
    $output = @(& git -C $Path @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed: $($output -join ' ')"
    }
    (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Fails {
    param([scriptblock]$Action, [string]$Pattern, [string]$Message)
    try {
        $null = & $Action
    } catch {
        if ($_.Exception.Message -like $Pattern) { return }
        throw "$Message Unexpected error: $($_.Exception.Message)"
    }
    throw "$Message The operation unexpectedly succeeded."
}

function Get-OutputAssignment {
    param([object[]]$Output, [string]$Name)
    $prefix = "$Name="
    $line = $Output | ForEach-Object { [string]$_ } |
        Where-Object { $_.StartsWith($prefix, [StringComparison]::Ordinal) } |
        Select-Object -Last 1
    if ($null -eq $line) { throw "Missing output assignment: $Name" }
    $line.Substring($prefix.Length)
}

function Remove-TestScratch {
    param([string]$Path, [string]$Root)
    $pathFull = [IO.Path]::GetFullPath($Path)
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    if (-not $pathFull.StartsWith(
        $rootFull + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Test scratch escaped its test root: $pathFull"
    }
    if (Test-Path -LiteralPath $pathFull) {
        Remove-Item -LiteralPath $pathFull -Recurse -Force
    }
}

$tempRoot = [IO.Path]::GetTempPath()
$testRoot = Join-Path $tempRoot ('elon-external-artifacts-test-' + [Guid]::NewGuid().ToString('N'))
$scratchRoot = Join-Path $testRoot 'scratch-root'
$oldLocalAppData = $env:LOCALAPPDATA
$oldScratchRoot = $env:ELON_AI_TASK_RUST_SCRATCH_ROOT
New-Item -ItemType Directory -Path $testRoot | Out-Null

try {
    $env:LOCALAPPDATA = Join-Path $testRoot 'local-appdata'
    $env:ELON_AI_TASK_RUST_SCRATCH_ROOT = $scratchRoot
    $origin = Join-Path $testRoot 'origin.git'
    $repo = Join-Path $testRoot 'repo'
    & git init --bare $origin *> $null
    if ($LASTEXITCODE -ne 0) { throw 'git init --bare failed.' }
    & git init -b main $repo *> $null
    if ($LASTEXITCODE -ne 0) { throw 'git init failed.' }
    Invoke-Git $repo @('config', 'user.email', 'artifact-test@example.invalid') | Out-Null
    Invoke-Git $repo @('config', 'user.name', 'artifact-test') | Out-Null
    Set-Content -LiteralPath (Join-Path $repo 'README.md') -Value 'fixture' -Encoding UTF8
    Invoke-Git $repo @('add', 'README.md') | Out-Null
    Invoke-Git $repo @('commit', '-m', 'seed artifact fixture') | Out-Null
    Invoke-Git $repo @('remote', 'add', 'origin', $origin) | Out-Null

    . (Join-Path $repoRoot 'scripts\ai-task-finish-contract.ps1')
    . (Join-Path $repoRoot 'scripts\ai-task-external-artifacts.ps1')
    $contractA = New-AiTaskFinishContract -RepoPath $repo
    $contractB = New-AiTaskFinishContract -RepoPath $repo
    $validatedA = Assert-AiTaskFinishContract -RepoPath $repo -ContractId $contractA
    $validatedB = Assert-AiTaskFinishContract -RepoPath $repo -ContractId $contractB

    $wrapperOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File `
        (Join-Path $repoRoot 'scripts\new-ai-task-rust-scratch.ps1') `
        -TaskWorktree $repo -TaskContract $contractA -Purpose 'wrapper-check' 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "Scratch wrapper failed: $($wrapperOutput -join ' ')" }
    $firstPath = Get-OutputAssignment -Output $wrapperOutput -Name 'AI_TASK_RUST_SCRATCH'
    $firstCache = Get-OutputAssignment -Output $wrapperOutput -Name 'ELON_RUST_CACHE_ROOT'
    $firstTarget = Get-OutputAssignment -Output $wrapperOutput -Name 'CARGO_TARGET_DIR'
    Assert-True (Test-Path -LiteralPath $firstCache -PathType Container) 'Wrapper cache path was not created.'
    Assert-True (Test-Path -LiteralPath $firstTarget -PathType Container) 'Wrapper target path was not created.'

    $second = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'wrapper-check'
    $other = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractB -Purpose 'other-contract'
    Assert-True ($firstPath -ne $second.scratch_path) 'Repeated purposes must allocate unique scratch paths.'

    $unknown = Join-Path $scratchRoot 'historical-unknown'
    New-Item -ItemType Directory -Path $unknown | Out-Null
    Set-Content -LiteralPath (Join-Path $unknown 'owner.txt') -Value 'preserve' -Encoding UTF8

    Assert-Fails { New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose '' } `
        '*purpose*' 'Empty scratch purpose must fail closed.'
    Assert-Fails { New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose ('x' * 65) } `
        '*purpose*' 'Oversized scratch purpose must fail closed.'
    Assert-Fails { New-AiTaskRustScratch -RepoPath $testRoot -ContractId $contractA -Purpose 'wrong-worktree' } `
        '*worktree identity mismatch*' 'Wrong worktree must fail closed.'
    Assert-Fails { New-AiTaskRustScratch -RepoPath $repo -ContractId ('0' * 64) -Purpose 'wrong-contract' } `
        '*contract not found*' 'Unknown contract must fail closed.'
    $driveOutsideTemp = Join-Path ([IO.Path]::GetPathRoot($testRoot)) 'elon-outside-temp-test'
    Assert-Fails { Get-AiTaskRustScratchRoot -ExplicitRoot $driveOutsideTemp } `
        '*under the operating-system temp root*' 'Scratch root outside system temp must fail closed.'
    $rootLinkTarget = Join-Path $testRoot 'root-link-target'
    $rootLink = Join-Path $testRoot 'root-link'
    New-Item -ItemType Directory -Path $rootLinkTarget | Out-Null
    New-Item -ItemType Junction -Path $rootLink -Target $rootLinkTarget | Out-Null
    Assert-Fails {
        New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'linked-root' `
            -ScratchRoot (Join-Path $rootLink 'scratch')
    } '*path chain contains a reparse point*' 'Linked scratch root ancestors must fail closed.'
    [IO.Directory]::Delete([IO.Path]::GetFullPath($rootLink), $false)

    $skipResult = Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA `
        -ValidatedContract $validatedA -Skip
    Assert-True ($skipResult.skipped -and (Test-Path -LiteralPath $firstPath)) `
        'Skip cleanup must preserve task scratch.'

    $cleanResult = Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA `
        -ValidatedContract $validatedA
    Assert-True ($cleanResult.removed_count -eq 2) 'Cleanup must remove every scratch owned by the exact contract.'
    Assert-True (-not (Test-Path -LiteralPath $firstPath)) 'Exact-contract scratch still exists after cleanup.'
    Assert-True (Test-Path -LiteralPath $other.scratch_path -PathType Container) `
        'Cleanup removed another contract scratch.'
    Assert-True (Test-Path -LiteralPath $unknown -PathType Container) 'Cleanup removed unknown adjacent data.'
    Assert-True (Test-Path -LiteralPath $scratchRoot -PathType Container) 'Cleanup removed the fixed root.'

    $tampered = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'tampered'
    $tamperedMarkerPath = Join-Path $tampered.scratch_path '.elon-ai-task-rust-scratch.json'
    $tamperedMarker = Get-Content -Raw -LiteralPath $tamperedMarkerPath | ConvertFrom-Json
    $tamperedMarker.purpose = 'changed'
    [IO.File]::WriteAllText(
        $tamperedMarkerPath,
        ($tamperedMarker | ConvertTo-Json -Depth 4 -Compress),
        [Text.UTF8Encoding]::new($false)
    )
    Assert-Fails {
        Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -ValidatedContract $validatedA
    } '*does not match its marker*' 'Tampered marker must block cleanup.'
    Assert-True (Test-Path -LiteralPath $tampered.scratch_path) 'Tampered scratch was deleted.'
    Remove-TestScratch -Path $tampered.scratch_path -Root $scratchRoot

    $unknownMember = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'unknown-member'
    Set-Content -LiteralPath (Join-Path $unknownMember.scratch_path 'unexpected.txt') -Value 'preserve' -Encoding UTF8
    Assert-Fails {
        Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -ValidatedContract $validatedA
    } '*unknown top-level members*' 'Unknown scratch members must block cleanup.'
    Assert-True (Test-Path -LiteralPath $unknownMember.scratch_path) 'Unknown-member scratch was deleted.'
    Remove-TestScratch -Path $unknownMember.scratch_path -Root $scratchRoot

    $missingMarker = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'missing-marker'
    Remove-Item -LiteralPath (Join-Path $missingMarker.scratch_path '.elon-ai-task-rust-scratch.json') -Force
    Assert-Fails {
        Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -ValidatedContract $validatedA
    } '*marker is missing*' 'Missing marker must block cleanup.'
    Assert-True (Test-Path -LiteralPath $missingMarker.scratch_path) 'Markerless scratch was deleted.'
    Remove-TestScratch -Path $missingMarker.scratch_path -Root $scratchRoot

    $reparse = New-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -Purpose 'reparse'
    $reparseTarget = Join-Path $testRoot 'reparse-target'
    New-Item -ItemType Directory -Path $reparseTarget | Out-Null
    Set-Content -LiteralPath (Join-Path $reparseTarget 'external.txt') -Value 'preserve' -Encoding UTF8
    $reparseLink = Join-Path $reparse.cache_root 'link'
    New-Item -ItemType Junction -Path $reparseLink -Target $reparseTarget | Out-Null
    Assert-Fails {
        Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractA -ValidatedContract $validatedA
    } '*reparse point*' 'Reparse points must block cleanup.'
    Assert-True (Test-Path -LiteralPath (Join-Path $reparseTarget 'external.txt')) `
        'Reparse rejection changed the external target.'
    $reparseLinkFull = [IO.Path]::GetFullPath($reparseLink)
    $reparseCacheFull = [IO.Path]::GetFullPath($reparse.cache_root).TrimEnd('\', '/')
    if (-not $reparseLinkFull.StartsWith(
        $reparseCacheFull + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw 'Reparse test link escaped its scratch cache directory.'
    }
    [IO.Directory]::Delete($reparseLinkFull, $false)
    Remove-TestScratch -Path $reparse.scratch_path -Root $scratchRoot

    $otherResult = Clear-AiTaskRustScratch -RepoPath $repo -ContractId $contractB `
        -ValidatedContract $validatedB
    Assert-True ($otherResult.removed_count -eq 1) 'Other contract cleanup count was incorrect.'
    Assert-True (Test-Path -LiteralPath $unknown -PathType Container) `
        'Final cleanup removed unknown adjacent data.'

    Write-Host 'PASS ai-task external Rust scratch lifecycle'
} finally {
    if ($null -eq $oldScratchRoot) {
        Remove-Item Env:ELON_AI_TASK_RUST_SCRATCH_ROOT -ErrorAction SilentlyContinue
    } else {
        $env:ELON_AI_TASK_RUST_SCRATCH_ROOT = $oldScratchRoot
    }
    if ($null -eq $oldLocalAppData) {
        Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue
    } else {
        $env:LOCALAPPDATA = $oldLocalAppData
    }
    $resolved = Resolve-Path -LiteralPath $testRoot -ErrorAction SilentlyContinue
    if ($null -ne $resolved) {
        $resolvedPath = $resolved.Path
        if ($resolvedPath.StartsWith([IO.Path]::GetFullPath($tempRoot), [StringComparison]::OrdinalIgnoreCase) -and
            (Split-Path -Leaf $resolvedPath).StartsWith('elon-external-artifacts-test-')) {
            Remove-Item -LiteralPath $resolvedPath -Recurse -Force
        } else {
            Write-Warning "Skip cleanup for unexpected test path: $resolvedPath"
        }
    }
}

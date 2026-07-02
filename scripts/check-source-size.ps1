<#
.SYNOPSIS
    Guard against growing source files into or inside the red zone.

.DESCRIPTION
    This is an incremental guard. Historical giant files may exist, but changed
    source files must not become red-zone files, and existing red-zone files must
    not grow. Use -AllowRedGrowth only for an explicit, reviewed exception.
#>
param(
    [string]$BaseRef = "origin/main",
    [switch]$AllowRedGrowth,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

$SourceExtensions = @(
    ".rs", ".kt", ".kts", ".java", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".ps1", ".psm1", ".sh",
    ".py", ".go", ".swift", ".cs", ".c", ".cc", ".cpp", ".h", ".hpp"
)
$SkipDirs = @(".git", ".gradle", ".idea", ".next", ".nuxt", ".venv", "build", "dist", "node_modules", "out", "target", "vendor")

function Stop-Guard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-LineCountFromText {
    param([AllowNull()][string[]]$Lines)
    if ($null -eq $Lines) { return 0 }
    return $Lines.Count
}

function Get-FileLineCount {
    param([string]$Path)
    return ([System.IO.File]::ReadLines($Path) | Measure-Object).Count
}

function Get-GitFileLineCount {
    param([string]$Ref, [string]$Path)
    $spec = "${Ref}:$Path"
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & git show $spec 2>$null
        if ($LASTEXITCODE -ne 0) { return $null }
        return Get-LineCountFromText $output
    } finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Test-SourcePath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    $normalized = $Path -replace "\\", "/"
    foreach ($segment in ($normalized -split "/")) {
        if ($SkipDirs -contains $segment) { return $false }
    }
    $name = [System.IO.Path]::GetFileName($normalized)
    if ($name.EndsWith(".min.js") -or $name.EndsWith(".generated.ts")) { return $false }
    $extension = [System.IO.Path]::GetExtension($normalized).ToLowerInvariant()
    return $SourceExtensions -contains $extension
}

function Get-SourceRole {
    param([string]$Path)
    $normalized = $Path -replace "\\", "/"
    $name = [System.IO.Path]::GetFileName($normalized)
    if (@("main.rs", "router.rs", "App.tsx", "App.jsx", "MainActivity.kt") -contains $name) {
        return "entry"
    }
    if ($name.EndsWith("Test.kt") -or $name.EndsWith("_test.rs") -or $name.EndsWith(".test.ts") -or $name.EndsWith(".spec.ts") -or $normalized.Contains("/test/") -or $normalized.Contains("/tests/")) {
        return "test"
    }
    $lower = $name.ToLowerInvariant()
    if ($lower.Contains("schema") -or $lower.Contains("types") -or $lower.EndsWith(".d.ts")) {
        return "schema"
    }
    if ($lower.Contains("helper") -or $lower.Contains("util") -or $lower -eq "common.rs" -or $lower -eq "common.ts") {
        return "helper"
    }
    return "source"
}

function Get-RedLimit {
    param([string]$Path)
    switch (Get-SourceRole $Path) {
        "helper" { return 600 }
        "schema" { return 1000 }
        "test" { return 1000 }
        default { return 800 }
    }
}

function Get-ChangedSourcePaths {
    param([string]$MergeBase)
    $paths = New-Object System.Collections.Generic.HashSet[string]
    $diff = & git diff --name-status --diff-filter=ACMR $MergeBase --
    if ($LASTEXITCODE -ne 0) {
        Stop-Guard "git diff failed while collecting changed source files."
    }
    foreach ($line in $diff) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $parts = $line -split "`t"
        $path = $parts[$parts.Count - 1]
        if (Test-SourcePath $path) {
            $null = $paths.Add(($path -replace "\\", "/"))
        }
    }
    return @($paths)
}

function Invoke-SourceSizeGuard {
    param([string]$Base)

    $repoRoot = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
        Stop-Guard "Current directory is not inside a git repository."
    }
    Set-Location $repoRoot

    & git rev-parse --verify $Base *> $null
    if ($LASTEXITCODE -ne 0) {
        Stop-Guard "Cannot verify base ref '$Base'. Run git fetch origin main first."
    }
    $mergeBase = (& git merge-base HEAD $Base).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($mergeBase)) {
        Stop-Guard "Cannot calculate merge-base between HEAD and '$Base'."
    }

    $changed = Get-ChangedSourcePaths $mergeBase
    if ($changed.Count -eq 0) {
        Write-Host "SOURCE_SIZE_GUARD=passed no_changed_source_files"
        return
    }

    $failures = @()
    $warnings = @()
    foreach ($path in ($changed | Sort-Object)) {
        $fullPath = Join-Path $repoRoot $path
        if (-not (Test-Path -LiteralPath $fullPath)) { continue }
        $currentLines = Get-FileLineCount $fullPath
        $baseLines = Get-GitFileLineCount $mergeBase $path
        $redLimit = Get-RedLimit $path
        $isNew = $null -eq $baseLines

        if ($isNew) {
            if ($currentLines -gt $redLimit) {
                $failures += "$path is new and has $currentLines lines, above red limit $redLimit."
            } elseif ($currentLines -gt 500) {
                $warnings += "$path is new and has $currentLines lines; keep it single-responsibility or split before it reaches red zone."
            }
            continue
        }

        if ($baseLines -le $redLimit -and $currentLines -gt $redLimit) {
            $failures += "$path crossed into red zone: $baseLines -> $currentLines lines (limit $redLimit)."
            continue
        }
        if (-not $AllowRedGrowth -and $baseLines -gt $redLimit -and $currentLines -gt $baseLines) {
            $failures += "$path is already red-zone and grew: $baseLines -> $currentLines lines (limit $redLimit). Extract a module first."
        }
    }

    foreach ($warning in $warnings) {
        Write-Host "SOURCE_SIZE_WARNING=$warning" -ForegroundColor Yellow
    }
    if ($failures.Count -gt 0) {
        Write-Host "SOURCE_SIZE_GUARD=failed" -ForegroundColor Red
        foreach ($failure in $failures) {
            Write-Host "  $failure" -ForegroundColor Red
        }
        Stop-Guard "Source-size guard failed. Do not grow giant files; extract focused modules first."
    }
    Write-Host "SOURCE_SIZE_GUARD=passed checked=$($changed.Count)"
}

function Invoke-SelfTestCase {
    param([string]$Repo, [string]$Base)
    Push-Location $Repo
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $PSCommandPath -BaseRef $Base *> $null
        return $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
        Pop-Location
    }
}

function Invoke-SelfTest {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ("elon_source_size_guard_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $root | Out-Null
    try {
        Push-Location $root
        git init *> $null
        git config user.email "guard@example.invalid"
        git config user.name "Source Size Guard"
        New-Item -ItemType Directory -Path "src" | Out-Null
        Set-Content -Path "src/legacy.rs" -Value (("fn old() {}" + [Environment]::NewLine) * 900) -NoNewline
        git add src/legacy.rs
        git commit -m "baseline legacy red file" *> $null
        $base = (git rev-parse HEAD).Trim()

        Set-Content -Path "src/legacy.rs" -Value (("fn old() {}" + [Environment]::NewLine) * 901) -NoNewline
        git add src/legacy.rs
        git commit -m "grow red file" *> $null
        if ((Invoke-SelfTestCase $root $base) -eq 0) { Stop-Guard "SelfTest failed: red growth was allowed." }

        git reset --hard $base *> $null
        Set-Content -Path "src/legacy.rs" -Value (("fn old() {}" + [Environment]::NewLine) * 850) -NoNewline
        git add src/legacy.rs
        git commit -m "shrink red file" *> $null
        if ((Invoke-SelfTestCase $root $base) -ne 0) { Stop-Guard "SelfTest failed: red shrink was blocked." }

        git reset --hard $base *> $null
        Set-Content -Path "src/new_big.rs" -Value (("fn new_big() {}" + [Environment]::NewLine) * 801) -NoNewline
        git add src/new_big.rs
        git commit -m "add new giant file" *> $null
        if ((Invoke-SelfTestCase $root $base) -eq 0) { Stop-Guard "SelfTest failed: new giant file was allowed." }

        Write-Host "SOURCE_SIZE_GUARD_SELFTEST=passed" -ForegroundColor Green
    } finally {
        Pop-Location
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-SelfTest
    exit 0
}

Invoke-SourceSizeGuard -Base $BaseRef

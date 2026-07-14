param(
    [string[]]$Roots = @(".github/prompts", ".github/agents", ".github/skills"),
    [int]$MaxLines = 120,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

$heavyContextPatterns = @(
    "copilot-instructions\.md",
    "git-deploy-workflow\.instructions\.md",
    "modular-architecture\.instructions\.md",
    "pc-frontend-migration\.instructions\.md",
    "apk-web-ui-sync\.instructions\.md",
    "docs[\\/]ai-agent-workflow\.md",
    "docs[\\/]system-architecture\.md",
    "docs[\\/]android-setup\.md",
    "docs[\\/]Design\.md"
)

$hardLoadPattern = "(?is)(Read these files before acting|先读取|必须先读).*(copilot-instructions|git-deploy-workflow|modular-architecture|ai-agent-workflow|system-architecture)"
$duplicatedLifecyclePatterns = @(
    'scripts[\\/]ai-task-preflight\.ps1 -CreateWorktree',
    'git pull --ff-only origin main',
    'scripts[\\/]cleanup-task-worktrees\.ps1 -Apply'
)

function Get-RelativePath {
    param([string]$Path)
    $root = (Get-Location).Path.TrimEnd("\", "/")
    if ($Path.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Path.Substring($root.Length + 1)
    }
    return $Path
}

$files = @()
foreach ($root in $Roots) {
    if (Test-Path $root) {
        $files += Get-ChildItem -Path $root -Recurse -File |
            Where-Object { $_.Extension -in @(".md", ".yaml", ".yml") }
    }
}

$rows = @()
$failures = @()

foreach ($file in ($files | Sort-Object FullName)) {
    $text = Get-Content -LiteralPath $file.FullName -Raw
    $lines = if ($text.Length -eq 0) { 0 } else { ($text -split "`r?`n").Count }
    $heavyRefs = 0
    foreach ($pattern in $heavyContextPatterns) {
        $matches = [regex]::Matches($text, $pattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        $heavyRefs += $matches.Count
    }
    $hardLoad = $text -match $hardLoadPattern
    $lifecycleDuplicates = 0
    foreach ($pattern in $duplicatedLifecyclePatterns) {
        $lifecycleDuplicates += [regex]::Matches(
            $text,
            $pattern,
            [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
        ).Count
    }
    $approxTokens = [int][math]::Ceiling($text.Length / 4.0)
    $relative = Get-RelativePath $file.FullName

    $status = "ok"
    $notes = @()
    if ($lines -gt $MaxLines) {
        $status = "fail"
        $notes += "lines>$MaxLines"
    }
    if ($hardLoad) {
        $status = "fail"
        $notes += "hard-coded-full-context"
    }
    if ($heavyRefs -gt 0 -and -not $hardLoad) {
        $notes += "routed-heavy-refs=$heavyRefs"
    }
    if ($lifecycleDuplicates -gt 0) {
        $status = "fail"
        $notes += "duplicated-lifecycle-commands=$lifecycleDuplicates"
    }

    $row = [pscustomobject]@{
        Path = $relative
        Lines = $lines
        ApproxTokens = $approxTokens
        HeavyRefs = $heavyRefs
        LifecycleDupes = $lifecycleDuplicates
        Status = $status
        Notes = ($notes -join ",")
    }
    $rows += $row
    if ($status -eq "fail") {
        $failures += $row
    }
}

if ($Json) {
    $rows | ConvertTo-Json -Depth 3
} else {
    $rows | Format-Table -AutoSize
}

$totalTokens = ($rows | Measure-Object -Property ApproxTokens -Sum).Sum
if ($null -eq $totalTokens) {
    $totalTokens = 0
}

if ($failures.Count -gt 0) {
    Write-Host "AI_PROMPT_ASSET_AUDIT=failed checked=$($rows.Count) totalApproxTokens=$totalTokens"
    foreach ($failure in $failures) {
        Write-Host "  $($failure.Path): $($failure.Notes)"
    }
    exit 1
}

Write-Host "AI_PROMPT_ASSET_AUDIT=passed checked=$($rows.Count) totalApproxTokens=$totalTokens"

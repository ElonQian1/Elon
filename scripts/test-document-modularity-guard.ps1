[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function New-TestRepo {
    param(
        [string]$Root,
        [int]$BaselineLines = 20
    )
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "docs") | Out-Null
    & git -C $Root init --quiet
    & git -C $Root config user.email "document-guard@example.invalid"
    & git -C $Root config user.name "Document Guard"
    $content = (1..$BaselineLines | ForEach-Object { "baseline line $_" }) -join "`n"
    [System.IO.File]::WriteAllText((Join-Path $Root "docs\current.md"), $content)
    & git -C $Root add docs/current.md
    & git -C $Root commit --quiet -m "baseline"
    return (& git -C $Root rev-parse HEAD).Trim()
}

function Invoke-Guard {
    param(
        [string]$Repo,
        [string]$Base,
        [switch]$Staged,
        [switch]$AutomationHandoff
    )
    $arguments = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", $guardPath,
        "-BaseRef", $Base
    )
    if ($Staged) { $arguments += "-Staged" }
    if ($AutomationHandoff) { $arguments += "-AutomationHandoff" }
    Push-Location $Repo
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell @arguments 2>&1 | Out-String
        $code = $LASTEXITCODE
        return [pscustomobject]@{ Code = $code; Output = $output }
    } finally {
        $ErrorActionPreference = $oldPreference
        Pop-Location
    }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$guardPath = Join-Path $repoRoot "scripts\check-document-modularity.ps1"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-document-guard-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempRoot | Out-Null

try {
    $formalRepo = Join-Path $tempRoot "formal"
    $formalBase = New-TestRepo $formalRepo
    $giantFormal = (1..801 | ForEach-Object { "formal line $_" }) -join "`n"
    [System.IO.File]::WriteAllText((Join-Path $formalRepo "docs\new-giant.md"), $giantFormal)
    & git -C $formalRepo add docs/new-giant.md
    $formal = Invoke-Guard $formalRepo $formalBase
    Assert-True ($formal.Code -ne 0) "new giant formal document was not blocked"
    Assert-True ($formal.Output -match "SPLIT_REQUIRED") "formal failure did not return split guidance: $($formal.Output)"

    $sourceRepo = Join-Path $tempRoot "source"
    $sourceBase = New-TestRepo $sourceRepo
    New-Item -ItemType Directory -Force -Path (Join-Path $sourceRepo "docs\inbox\conversations") | Out-Null
    $sourceContent = (1..1000 | ForEach-Object { "source line $_" }) -join "`n"
    [System.IO.File]::WriteAllText(
        (Join-Path $sourceRepo "docs\inbox\conversations\long-discussion.md"),
        $sourceContent
    )
    & git -C $sourceRepo add docs/inbox/conversations/long-discussion.md
    $source = Invoke-Guard $sourceRepo $sourceBase
    Assert-True ($source.Code -eq 0) "long source material should be preserved: $($source.Output)"
    Assert-True ($source.Output -match "DOCUMENT_MODULARITY_WARNING") "source material warning was not emitted"

    $headingRepo = Join-Path $tempRoot "headings"
    $headingBase = New-TestRepo $headingRepo
    $headingContent = (1..41 | ForEach-Object { "## Responsibility $_`ncontent" }) -join "`n"
    [System.IO.File]::WriteAllText((Join-Path $headingRepo "docs\many-headings.md"), $headingContent)
    & git -C $headingRepo add docs/many-headings.md
    $heading = Invoke-Guard $headingRepo $headingBase
    Assert-True ($heading.Code -ne 0) "mixed-responsibility heading count was not blocked"

    $growthRepo = Join-Path $tempRoot "growth"
    $growthBase = New-TestRepo $growthRepo -BaselineLines 801
    Add-Content -LiteralPath (Join-Path $growthRepo "docs\current.md") -Value "`nmore"
    $growth = Invoke-Guard $growthRepo $growthBase
    Assert-True ($growth.Code -ne 0) "existing giant formal document growth was not blocked"

    $promotionRepo = Join-Path $tempRoot "promotion"
    $null = New-TestRepo $promotionRepo
    New-Item -ItemType Directory -Force -Path (Join-Path $promotionRepo "docs\inbox") | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $promotionRepo "docs\inbox\source.md"), $sourceContent)
    & git -C $promotionRepo add docs/inbox/source.md
    & git -C $promotionRepo commit --quiet -m "add source material"
    $promotionBase = (& git -C $promotionRepo rev-parse HEAD).Trim()
    & git -C $promotionRepo mv docs/inbox/source.md docs/promoted.md
    $promotion = Invoke-Guard $promotionRepo $promotionBase
    Assert-True ($promotion.Code -ne 0) "giant source material was promoted to a formal path without splitting"

    $stagedRepo = Join-Path $tempRoot "staged"
    $null = New-TestRepo $stagedRepo
    [System.IO.File]::WriteAllText((Join-Path $stagedRepo "docs\staged-giant.md"), $giantFormal)
    & git -C $stagedRepo add docs/staged-giant.md
    $staged = Invoke-Guard $stagedRepo "HEAD" -Staged
    Assert-True ($staged.Code -ne 0) "staged giant document was not blocked before commit"

    $handoffRepo = Join-Path $tempRoot "handoff"
    $null = New-TestRepo $handoffRepo
    [System.IO.File]::WriteAllText((Join-Path $handoffRepo "docs\handoff-giant.md"), $giantFormal)
    & git -C $handoffRepo add docs/handoff-giant.md
    $handoff = Invoke-Guard $handoffRepo "HEAD" -Staged -AutomationHandoff
    Assert-True ($handoff.Code -eq 0) "pre-commit automation handoff should allow the local commit: $($handoff.Output)"
    Assert-True ($handoff.Output -match "deferred_to_post_commit_automation") "handoff did not report deferred automation"
    $signalPath = (& git -C $handoffRepo rev-parse --git-path "elon/document-organization-trigger.json").Trim()
    if (-not [System.IO.Path]::IsPathRooted($signalPath)) {
        $signalPath = Join-Path $handoffRepo $signalPath
    }
    Assert-True (Test-Path -LiteralPath $signalPath -PathType Leaf) "handoff signal was not persisted"
    $signal = [System.IO.File]::ReadAllText($signalPath) | ConvertFrom-Json
    Assert-True ($signal.severity -eq "blocking") "handoff severity was not blocking"
    Assert-True (@($signal.paths) -contains "docs/handoff-giant.md") "handoff signal omitted the changed document"

    $warningHandoffRepo = Join-Path $tempRoot "warning-handoff"
    $warningBase = New-TestRepo $warningHandoffRepo
    New-Item -ItemType Directory -Force -Path (Join-Path $warningHandoffRepo "docs\inbox") | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $warningHandoffRepo "docs\inbox\source.md"), $sourceContent)
    & git -C $warningHandoffRepo add docs/inbox/source.md
    $warningHandoff = Invoke-Guard $warningHandoffRepo $warningBase -Staged -AutomationHandoff
    Assert-True ($warningHandoff.Code -eq 0) "source warning handoff failed: $($warningHandoff.Output)"
    $warningSignalPath = (& git -C $warningHandoffRepo rev-parse --git-path "elon/document-organization-trigger.json").Trim()
    if (-not [System.IO.Path]::IsPathRooted($warningSignalPath)) {
        $warningSignalPath = Join-Path $warningHandoffRepo $warningSignalPath
    }
    $warningSignal = [System.IO.File]::ReadAllText($warningSignalPath) | ConvertFrom-Json
    Assert-True ($warningSignal.severity -eq "warning") "source material did not produce a warning trigger"

    $preCommit = [System.IO.File]::ReadAllText((Join-Path $repoRoot ".githooks\pre-commit"))
    $postCommit = [System.IO.File]::ReadAllText((Join-Path $repoRoot ".githooks\post-commit"))
    $prePush = [System.IO.File]::ReadAllText((Join-Path $repoRoot ".githooks\pre-push"))
    Assert-True ($preCommit.Contains("check-document-modularity.ps1")) "pre-commit hook does not run the document guard"
    Assert-True ($preCommit.Contains("-AutomationHandoff")) "pre-commit hook does not enable automatic handoff"
    Assert-True ($postCommit.Contains("dispatch-document-organization.ps1")) "post-commit hook does not dispatch the persisted signal"
    Assert-True ($prePush.Contains("check-document-modularity.ps1")) "pre-push hook does not repeat the document guard"
    Assert-True (-not $prePush.Contains("-AutomationHandoff")) "pre-push must keep the strict document gate"

    Write-Host "DOCUMENT_MODULARITY_GUARD_TEST=passed"
} finally {
    $resolvedTemp = [System.IO.Path]::GetFullPath($tempRoot)
    $resolvedSystemTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolvedTemp.StartsWith($resolvedSystemTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

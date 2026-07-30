<#
.SYNOPSIS
    Prevent current project documentation from growing into giant files.

.DESCRIPTION
    This incremental guard checks changed Markdown files by line count, UTF-8
    byte size, and heading count. Source material such as inbox discussions and
    drafts is preserved and only warned about. Current documentation must stay
    modular: new red-zone files, red-zone crossings, and growth inside an
    existing red zone fail the guard.
#>
param(
    [string]$BaseRef = "origin/main",
    [switch]$Staged,
    [switch]$AllowFormalGrowth,
    [int]$MaxLines = 800,
    [int]$MaxBytes = 50000,
    [int]$MaxHeadings = 40
)

$ErrorActionPreference = "Stop"
$SkipDirs = @(
    ".git", ".gradle", ".idea", ".next", ".nuxt", ".venv", ".ai-tmp",
    "build", "dist", "node_modules", "out", "target", "vendor"
)
$SourcePathSegments = @(
    "archive", "archives", "conversation", "conversations", "draft", "drafts",
    "historical", "history", "inbox", "report", "reports", "transcript", "transcripts"
)

function Stop-DocumentGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function ConvertTo-NormalizedPath {
    param([string]$Path)
    return (($Path -replace "\\", "/").TrimStart([char[]]@(".", "/"))).ToLowerInvariant()
}

function Test-DocumentPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    $normalized = ConvertTo-NormalizedPath $Path
    foreach ($segment in ($normalized -split "/")) {
        if ($SkipDirs -contains $segment) { return $false }
    }
    $extension = [System.IO.Path]::GetExtension($normalized)
    return $extension -eq ".md" -or $extension -eq ".mdx"
}

function Get-ManifestSourcePaths {
    param([string]$RepoRoot)
    $paths = New-Object System.Collections.Generic.HashSet[string]
    $manifestPath = Join-Path $RepoRoot ".elon\document-sections.json"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        return ,$paths
    }
    try {
        $manifest = [System.IO.File]::ReadAllText($manifestPath, [System.Text.Encoding]::UTF8) |
            ConvertFrom-Json
        foreach ($property in @($manifest.governance_facets.PSObject.Properties)) {
            $facet = $property.Value
            $lifecycle = [string]$facet.lifecycle
            $documentType = [string]$facet.document_type
            if (
                $lifecycle -in @("archive", "archived", "draft", "historical", "source_material") -or
                $documentType -in @("archive", "discussion", "report", "source_material", "transcript")
            ) {
                $null = $paths.Add((ConvertTo-NormalizedPath $property.Name))
            }
        }
        foreach ($property in @($manifest.document_metadata.PSObject.Properties)) {
            $metadata = $property.Value
            $documentType = [string]$metadata.doc_type
            if ($documentType -in @("archive", "discussion", "report", "source_material", "transcript")) {
                $null = $paths.Add((ConvertTo-NormalizedPath $property.Name))
            }
        }
    } catch {
        Stop-DocumentGuard "Cannot parse .elon/document-sections.json: $($_.Exception.Message)"
    }
    return ,$paths
}

function Test-SourceMaterial {
    param(
        [string]$Path,
        [System.Collections.Generic.HashSet[string]]$ManifestSourcePaths
    )
    $normalized = ConvertTo-NormalizedPath $Path
    if ($ManifestSourcePaths.Contains($normalized)) { return $true }
    $segments = $normalized -split "/"
    if (@($segments | Where-Object { $SourcePathSegments -contains $_ }).Count -gt 0) {
        return $true
    }
    $name = [System.IO.Path]::GetFileNameWithoutExtension($normalized)
    return $name -match "(^|[-_.])(chat|conversation|discussion|report|transcript)([-_.]|$)"
}

function Get-DocumentMetrics {
    param([AllowNull()][string]$Content)
    if ($null -eq $Content) { return $null }
    $normalized = $Content -replace "`r`n?", "`n"
    $lines = if ($normalized.Length -eq 0) { 0 } else { ($normalized -split "`n").Count }
    $headings = 0
    $fenced = $false
    foreach ($line in ($normalized -split "`n")) {
        $trimmed = $line.TrimStart()
        if ($trimmed.StartsWith('```') -or $trimmed.StartsWith('~~~')) {
            $fenced = -not $fenced
            continue
        }
        if (-not $fenced -and $trimmed -match "^#{1,6}\s+\S") {
            $headings += 1
        }
    }
    return [pscustomobject]@{
        Lines = $lines
        Bytes = [System.Text.Encoding]::UTF8.GetByteCount($normalized)
        Headings = $headings
    }
}

function Get-GitContent {
    param([string]$Ref, [string]$Path)
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $lines = @(& git -c core.quotepath=false show "${Ref}:$Path" 2>$null)
        if ($LASTEXITCODE -ne 0) {
            $global:LASTEXITCODE = 0
            return $null
        }
        return ($lines -join "`n")
    } finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Get-ChangedDocuments {
    param([string]$DiffBase, [switch]$UseStaged)
    $arguments = @("-c", "core.quotepath=false", "diff")
    if ($UseStaged) { $arguments += "--cached" }
    $arguments += @("--name-status", "--diff-filter=ACMR", $DiffBase, "--")
    $diff = @(& git @arguments)
    if ($LASTEXITCODE -ne 0) {
        Stop-DocumentGuard "git diff failed while collecting changed documents."
    }
    $documents = @()
    foreach ($line in $diff) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $parts = $line -split "`t"
        $status = $parts[0]
        if ($status.StartsWith("R") -or $status.StartsWith("C")) {
            $basePath = $parts[1]
            $currentPath = $parts[2]
        } else {
            $basePath = $parts[1]
            $currentPath = $parts[1]
        }
        if (Test-DocumentPath $currentPath) {
            $documents += [pscustomobject]@{
                BasePath = $basePath -replace "\\", "/"
                CurrentPath = $currentPath -replace "\\", "/"
            }
        }
    }
    return $documents
}

function Test-RedZone {
    param($Metrics)
    return $null -ne $Metrics -and (
        $Metrics.Lines -gt $MaxLines -or
        $Metrics.Bytes -gt $MaxBytes -or
        $Metrics.Headings -gt $MaxHeadings
    )
}

function Format-Metrics {
    param($Metrics)
    if ($null -eq $Metrics) { return "new" }
    return "lines=$($Metrics.Lines), bytes=$($Metrics.Bytes), headings=$($Metrics.Headings)"
}

function Invoke-DocumentModularityGuard {
    $repoRoot = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
        Stop-DocumentGuard "Current directory is not inside a git repository."
    }
    Set-Location $repoRoot

    & git rev-parse --verify $BaseRef *> $null
    if ($LASTEXITCODE -ne 0) {
        Stop-DocumentGuard "Cannot verify base ref '$BaseRef'. Run git fetch origin main first."
    }
    $diffBase = if ($Staged) {
        $BaseRef
    } else {
        (& git merge-base HEAD $BaseRef).Trim()
    }
    if ([string]::IsNullOrWhiteSpace($diffBase)) {
        Stop-DocumentGuard "Cannot calculate document diff base for '$BaseRef'."
    }

    $changed = @(Get-ChangedDocuments -DiffBase $diffBase -UseStaged:$Staged)
    if ($changed.Count -eq 0) {
        Write-Host "DOCUMENT_MODULARITY_GUARD=passed no_changed_documents"
        return
    }

    $manifestSourcePaths = Get-ManifestSourcePaths $repoRoot
    $failures = @()
    $warnings = @()
    foreach ($document in $changed) {
        $fullPath = Join-Path $repoRoot $document.CurrentPath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) { continue }
        $current = Get-DocumentMetrics ([System.IO.File]::ReadAllText($fullPath, [System.Text.Encoding]::UTF8))
        $base = Get-DocumentMetrics (Get-GitContent -Ref $diffBase -Path $document.BasePath)
        $currentRed = Test-RedZone $current
        $baseRed = Test-RedZone $base
        $sourceMaterial = Test-SourceMaterial $document.CurrentPath $manifestSourcePaths
        $baseSourceMaterial = Test-SourceMaterial $document.BasePath $manifestSourcePaths

        if ($sourceMaterial) {
            if ($currentRed) {
                $warnings += "$($document.CurrentPath) is source material and may remain intact ($((Format-Metrics $current))). Compile accepted conclusions into focused current documents."
            }
            continue
        }

        $grew = $null -eq $base -or
            $current.Lines -gt $base.Lines -or
            $current.Bytes -gt $base.Bytes -or
            $current.Headings -gt $base.Headings
        if ($currentRed -and ($null -eq $base -or -not $baseRed -or $baseSourceMaterial)) {
            $failures += "$($document.CurrentPath) entered the formal-document red zone: $(Format-Metrics $base) -> $(Format-Metrics $current)."
        } elseif ($currentRed -and $baseRed -and $grew -and -not $AllowFormalGrowth) {
            $failures += "$($document.CurrentPath) is already a giant formal document and grew: $(Format-Metrics $base) -> $(Format-Metrics $current)."
        } elseif (
            $current.Lines -gt [math]::Floor($MaxLines * 0.75) -or
            $current.Bytes -gt [math]::Floor($MaxBytes * 0.75) -or
            $current.Headings -gt [math]::Floor($MaxHeadings * 0.75)
        ) {
            $warnings += "$($document.CurrentPath) is approaching the modularity limit ($(Format-Metrics $current))."
        }
    }

    foreach ($warning in $warnings) {
        Write-Host "DOCUMENT_MODULARITY_WARNING=$warning" -ForegroundColor Yellow
    }
    if ($failures.Count -gt 0) {
        Write-Host "DOCUMENT_MODULARITY_GUARD=failed" -ForegroundColor Red
        foreach ($failure in $failures) {
            Write-Host "  $failure" -ForegroundColor Red
        }
        Write-Host "SPLIT_REQUIRED=Keep a short README/index and move each responsibility into focused sibling documents." -ForegroundColor Red
        Write-Host "MCP_REVIEW=Run project_docs_review_modularity for the affected paths before committing again." -ForegroundColor Red
        Stop-DocumentGuard "Document modularity guard failed. Do not commit a growing giant formal document."
    }
    Write-Host "DOCUMENT_MODULARITY_GUARD=passed checked=$($changed.Count) warnings=$($warnings.Count)"
}

Invoke-DocumentModularityGuard

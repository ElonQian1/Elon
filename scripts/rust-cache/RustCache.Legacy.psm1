Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -Force -DisableNameChecking

function Get-RustCacheLegacyDirectorySize {
    param([Parameter(Mandatory)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) { return [int64]0 }
    $sum = (Get-ChildItem -LiteralPath $Path -File -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    if ($null -eq $sum) { return [int64]0 }
    return [int64]$sum
}

function Assert-RustCacheLegacyPurgePath {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$LegacyPath
    )

    if (-not (Test-RustCacheAbsolutePath $LegacyPath)) {
        throw "Legacy purge path must be absolute: $LegacyPath"
    }
    $fullPath = [System.IO.Path]::GetFullPath($LegacyPath).TrimEnd('\', '/')
    $rootPath = [System.IO.Path]::GetPathRoot($fullPath).TrimEnd('\', '/')
    $managedRoot = [System.IO.Path]::GetFullPath($CacheRoot).TrimEnd('\', '/')
    if ($fullPath -ieq $rootPath -or $fullPath -ieq $managedRoot) {
        throw "Refusing broad legacy purge path: $fullPath"
    }
    $leaf = Split-Path $fullPath -Leaf
    if ($leaf -ine "target" -and $leaf -ine "sccache" -and $leaf -notmatch '(?i)-target$') {
        throw "Legacy purge path must name a target or sccache directory: $fullPath"
    }
    if (Test-Path -LiteralPath $fullPath) {
        $item = Get-Item -LiteralPath $fullPath -Force
        if (-not $item.PSIsContainer) {
            throw "Legacy purge path is not a directory: $fullPath"
        }
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Refusing to recursively purge a reparse point: $fullPath"
        }
    }
    return $fullPath
}

function Write-RustCacheLegacyPurgeReport {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)]$Report
    )

    $reportRoot = Join-Path $CacheRoot "reports"
    New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
    $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss-fff")
    $path = Join-Path $reportRoot "legacy-purge-$stamp.json"
    $Report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $path -Encoding UTF8
    return $path
}

function Invoke-RustCacheLegacyPurge {
    param(
        [string]$CacheRoot,
        [string]$RepoRoot,
        [Parameter(Mandatory)][string]$LegacyPath,
        [switch]$Apply
    )

    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
    $fullPath = Assert-RustCacheLegacyPurgePath -CacheRoot $root -LegacyPath $LegacyPath
    $policyPath = Get-RustCachePolicyPath -CacheRoot $root
    $policy = Get-RustCachePolicy -CacheRoot $root
    $record = @($policy.legacy_caches) |
        Where-Object { [string]$_.path -ieq $fullPath } |
        Select-Object -First 1
    if (-not $record) {
        throw "Legacy cache is not registered in policy.json: $fullPath"
    }
    if (-not [bool]$record.retired) {
        throw "Legacy cache is registered but not retired: $fullPath"
    }

    $existsBefore = Test-Path -LiteralPath $fullPath
    $sizeBytes = if ($existsBefore) { Get-RustCacheLegacyDirectorySize -Path $fullPath } else { [int64]0 }
    if ($Apply) {
        $activeBuilds = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue)
        if ($activeBuilds.Count -gt 0) {
            $summary = ($activeBuilds | ForEach-Object { "$($_.ProcessName):$($_.Id)" }) -join ", "
            throw "Refusing legacy cache purge while Cargo/rustc processes are active: $summary"
        }
        if ($existsBefore) {
            Remove-Item -LiteralPath $fullPath -Recurse -Force -ErrorAction Stop
        }
        $record | Add-Member -NotePropertyName removed_utc -NotePropertyValue ([DateTime]::UtcNow.ToString("o")) -Force
        $record | Add-Member -NotePropertyName removed_bytes -NotePropertyValue ([int64]$sizeBytes) -Force
        $policy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $policyPath -Encoding UTF8
    }

    $report = [pscustomobject]@{
        schema_version = 1
        generated_utc = [DateTime]::UtcNow.ToString("o")
        mode = if ($Apply) { "apply" } else { "dry-run" }
        action = if ($Apply) { if ($existsBefore) { "deleted" } else { "already-missing" } } else { if ($existsBefore) { "would-delete" } else { "already-missing" } }
        label = [string]$record.label
        path = $fullPath
        size_bytes = [int64]$sizeBytes
        existed_before = [bool]$existsBefore
        exists_after = Test-Path -LiteralPath $fullPath
    }
    $reportPath = Write-RustCacheLegacyPurgeReport -CacheRoot $root -Report $report
    $report | Add-Member -NotePropertyName report_path -NotePropertyValue $reportPath
    return $report
}

Export-ModuleMember -Function Assert-RustCacheLegacyPurgePath, Write-RustCacheLegacyPurgeReport, Invoke-RustCacheLegacyPurge

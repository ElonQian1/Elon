function Read-ElonUtf8TextFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    $utf8 = [System.Text.UTF8Encoding]::new($false, $true)
    $text = $utf8.GetString([System.IO.File]::ReadAllBytes($Path))
    if ($text.Length -gt 0 -and $text[0] -eq [char]0xFEFF) { return $text.Substring(1) }
    $text
}

function Get-ElonRepositoryPathsFromRoot {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    $repoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
    $gitMarker = Join-Path $repoRoot '.git'
    if (Test-Path -LiteralPath $gitMarker -PathType Container) {
        $gitDir = [System.IO.Path]::GetFullPath($gitMarker)
    } elseif (Test-Path -LiteralPath $gitMarker -PathType Leaf) {
        $marker = (Read-ElonUtf8TextFile -Path $gitMarker).Trim()
        if ($marker -notmatch '^gitdir:\s*(.+)$') {
            throw "Invalid Git worktree marker: $gitMarker"
        }
        $gitDirValue = $Matches[1].Trim()
        $gitDir = if ([System.IO.Path]::IsPathRooted($gitDirValue)) {
            [System.IO.Path]::GetFullPath($gitDirValue)
        } else {
            [System.IO.Path]::GetFullPath((Join-Path $repoRoot $gitDirValue))
        }
    } else {
        throw "Repository Git marker is missing: $gitMarker"
    }

    $commonMarker = Join-Path $gitDir 'commondir'
    $gitCommonDir = if (Test-Path -LiteralPath $commonMarker -PathType Leaf) {
        $commonValue = (Read-ElonUtf8TextFile -Path $commonMarker).Trim()
        [System.IO.Path]::GetFullPath((Join-Path $gitDir $commonValue))
    } else {
        $gitDir
    }
    [PSCustomObject]@{
        RepoRoot = $repoRoot
        GitDir = $gitDir
        GitCommonDir = $gitCommonDir
    }
}

function Get-ElonRepositoryPathsFromScriptRoot {
    param([Parameter(Mandatory = $true)][string]$ScriptRoot)
    Get-ElonRepositoryPathsFromRoot -RepoRoot (Join-Path $ScriptRoot '..')
}

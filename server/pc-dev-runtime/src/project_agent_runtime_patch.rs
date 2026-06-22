pub(crate) fn agent_runtime_apply_patch_helpers() -> &'static str {
    r#"
function Normalize-AgentPatch {
    param([AllowNull()][string]$Patch)
    if ($null -eq $Patch) {
        $text = ''
    } else {
        $text = [string]$Patch
    }
    $trimmed = $text.Trim()
    if (-not $trimmed) {
        throw 'apply_patch denied by policy: patch cannot be empty'
    }
    if ($trimmed.Length -gt 60000) {
        throw 'apply_patch denied by policy: patch is too large; split it into smaller patches'
    }
    if ($trimmed.StartsWith('```')) {
        $lines = @($trimmed -split "`r?`n")
        $body = New-Object System.Collections.Generic.List[string]
        for ($i = 1; $i -lt $lines.Count; $i++) {
            if ($lines[$i].Trim() -eq '```') { break }
            $body.Add($lines[$i])
        }
        if ($body.Count -gt 0) {
            $trimmed = ($body -join "`n").Trim()
        }
    }
    if (-not $trimmed.Contains('@@') -or (-not $trimmed.Contains('diff --git ') -and -not $trimmed.Contains("--- `n") -and -not $trimmed.StartsWith('--- '))) {
        throw 'apply_patch denied by policy: patch must be unified diff'
    }
    if ($trimmed.Contains('GIT binary patch') -or $trimmed.Contains('Binary files ')) {
        throw 'apply_patch denied by policy: binary patches are not supported'
    }
    return ($trimmed.Replace("`r`n", "`n").Replace("`r", "`n") + "`n")
}

function Normalize-AgentPatchPath {
    param([AllowNull()][string]$RawPath)
    if ($null -eq $RawPath) { return '' }
    $path = ([string]$RawPath).Trim().Trim('"')
    if (-not $path -or $path -eq '/dev/null') { return '' }
    if ($path.StartsWith('a/')) { return $path.Substring(2) }
    if ($path.StartsWith('b/')) { return $path.Substring(2) }
    return $path
}

function Add-AgentPatchTouchedFile {
    param(
        [Parameter(Mandatory = $true)]$Files,
        [AllowNull()][string]$Path
    )
    $normalized = Normalize-AgentPatchPath $Path
    if ($normalized) {
        [void]$Files.Add($normalized)
    }
}

function Get-AgentPatchTouchedFiles {
    param([Parameter(Mandatory = $true)][string]$Patch)
    $files = New-Object System.Collections.Generic.List[string]
    foreach ($line in ($Patch -split "`n")) {
        if ($line.StartsWith('diff --git ')) {
            $parts = @($line -split '\s+')
            if ($parts.Count -ge 4) {
                Add-AgentPatchTouchedFile $files $parts[2]
                Add-AgentPatchTouchedFile $files $parts[3]
            }
        } elseif ($line.StartsWith('--- ')) {
            $path = @($line.Substring(4).Trim() -split '\s+')[0]
            Add-AgentPatchTouchedFile $files $path
        } elseif ($line.StartsWith('+++ ')) {
            $path = @($line.Substring(4).Trim() -split '\s+')[0]
            Add-AgentPatchTouchedFile $files $path
        }
    }
    $unique = @($files | Sort-Object -Unique)
    if ($unique.Count -eq 0) {
        throw 'apply_patch denied by policy: patch did not identify any target files'
    }
    return $unique
}

function Test-AgentPatchPathSafe {
    param([Parameter(Mandatory = $true)][string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        throw "apply_patch denied by policy: absolute patch path $Path"
    }
    $parts = @($Path -split '[\\/]' | Where-Object { $_ -and $_ -ne '.' })
    foreach ($part in $parts) {
        if ($part -eq '..') {
            throw "apply_patch denied by policy: parent path segment in $Path"
        }
        if ($part.Equals('.git', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "apply_patch denied by policy: patch cannot target .git"
        }
    }
    [void](Resolve-SafePath $Path)
}

function Invoke-GitApplyPatchFile {
    param(
        [Parameter(Mandatory = $true)][string]$PatchFile,
        [switch]$CheckOnly
    )
    if (-not (Test-Tool 'git')) {
        throw 'apply_patch failed: Git is not installed or not on PATH'
    }
    $gitArgs = @('-C', $ProjectRoot, 'apply', '--whitespace=nowarn')
    if ($CheckOnly) {
        $gitArgs += '--check'
    }
    $gitArgs += $PatchFile
    $output = & git @gitArgs 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "git apply failed with exit code $LASTEXITCODE`n$output"
    }
    return $output
}

function Invoke-AgentApplyPatch {
    param(
        [AllowNull()][string]$Patch,
        [switch]$CheckOnly
    )
    try {
        $normalized = Normalize-AgentPatch $Patch
        $files = @(Get-AgentPatchTouchedFiles $normalized)
        foreach ($file in $files) {
            Test-AgentPatchPathSafe $file
        }
    } catch {
        return [string]$_.Exception.Message
    }

    $patchFile = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-agent-patch-{0}-{1}.diff" -f $PID, [System.Guid]::NewGuid().ToString('N'))
    try {
        $encoding = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($patchFile, $normalized, $encoding)
        try {
            [void](Invoke-GitApplyPatchFile -PatchFile $patchFile -CheckOnly)
        } catch {
            return "apply_patch check failed: $($_.Exception.Message)"
        }

        $target = ($files -join ', ')
        if ($DryRun -or $CheckOnly) {
            return "apply_patch check ok: $target"
        }
        if (-not (Confirm-AgentAction 'apply_patch' $target)) {
            return "apply_patch denied by user: $target"
        }
        try {
            [void](Invoke-GitApplyPatchFile -PatchFile $patchFile)
        } catch {
            return "apply_patch failed: $($_.Exception.Message)"
        }
        return "apply_patch ok: $target"
    } finally {
        Remove-Item -LiteralPath $patchFile -Force -ErrorAction SilentlyContinue
    }
}
"#
}

pub(crate) fn agent_runtime_apply_patch_action_case() -> &'static str {
    r#"        'apply_patch' {
            $patch = [string]$Action.patch
            $checkOnly = [bool]$Action.check_only
            return Invoke-AgentApplyPatch -Patch $patch -CheckOnly:$checkOnly
        }
"#
}

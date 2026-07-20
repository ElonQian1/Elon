function Get-ValidationSha256 {
    param([Parameter(Mandatory)][string]$Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally { $sha.Dispose() }
}

function ConvertTo-ValidationCommand {
    param([Parameter(Mandatory)][string[]]$CargoArgs)
    $normalized = foreach ($arg in $CargoArgs) {
        ([string]$arg).Trim().Replace('\', '/').Replace('"', '\"')
    }
    return ($normalized -join "`n")
}

function Get-ValidationGitSnapshot {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $relevantPattern = '^(server/|rust-cache\.project\.json$|\.rustfmt-version$|scripts/(cargo-dev|validate-rust|format-rust|check-source-size)|scripts/(validation|rust-cache)/)'
    $index = @(& git -c core.quotepath=false -C $RepoRoot ls-files -s | Where-Object {
        $path = ($_ -split "`t",2)[-1].Replace('\','/'); $path -match $relevantPattern
    } | Sort-Object)
    if ($LASTEXITCODE -ne 0) { throw "Unable to enumerate the Git index snapshot." }
    $diff = @(& git -c core.quotepath=false -C $RepoRoot diff --binary --no-ext-diff -- server rust-cache.project.json .rustfmt-version scripts/cargo-dev.ps1 scripts/cargo-dev.sh scripts/validate-rust.ps1 scripts/validation scripts/rust-cache scripts/format-rust.ps1 scripts/format-rust.sh scripts/check-source-size.ps1)
    if ($LASTEXITCODE -ne 0) { throw "Unable to read the dirty workspace diff." }
    $untracked = @(& git -c core.quotepath=false -C $RepoRoot ls-files --others --exclude-standard | Where-Object { $_.Replace('\','/') -match $relevantPattern } | Sort-Object -Unique)
    $records = New-Object System.Collections.Generic.List[string]
    foreach ($relative in $untracked) {
        $path = Join-Path $RepoRoot $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
        $hash = (& git -C $RepoRoot hash-object --no-filters -- $relative).Trim()
        if ($LASTEXITCODE -ne 0) { throw "Unable to hash workspace file: $relative" }
        $records.Add("$($relative.Replace('\','/'))`t$hash")
    }
    $status = @(& git -c core.quotepath=false -C $RepoRoot status --porcelain=v1 --untracked-files=all | Where-Object {
        $path = if ($_.Length -gt 3) { $_.Substring(3).Replace('\','/') } else { "" }
        $path -match $relevantPattern
    } | Sort-Object)
    return [pscustomobject]@{
        index = @($index)
        untracked = @($records)
        status = @($status)
        digest = Get-ValidationSha256 -Text ((@("--index--") + $index + @("--dirty-diff--") + $diff + @("--untracked--") + @($records) + @("--status--") + $status) -join "`n")
    }
}

function Get-ValidationFingerprint {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string[]]$CargoArgs,
        [string]$Domain = "dev-windows-msvc",
        [string]$TargetDir
    )
    $snapshot = Get-ValidationGitSnapshot -RepoRoot $RepoRoot
    $lockPath = Join-Path $RepoRoot "server\Cargo.lock"
    if (-not (Test-Path -LiteralPath $lockPath)) { $lockPath = Join-Path $RepoRoot "Cargo.lock" }
    $lockHash = if (Test-Path -LiteralPath $lockPath) { (& git hash-object -- $lockPath).Trim() } else { "missing" }
    $rustc = ((& rustc -vV 2>&1) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0) { throw "rustc -vV failed while creating validation fingerprint." }
    $remote = ((& git -C $RepoRoot config --get remote.origin.url 2>$null) -join "").Trim().ToLowerInvariant()
    $command = ConvertTo-ValidationCommand -CargoArgs $CargoArgs
    $payload = [ordered]@{
        schema = "elon.validation.fingerprint.v1"
        project = $remote
        snapshot = $snapshot.digest
        cargo_lock = $lockHash
        rustc = $rustc
        domain = $Domain
        target_dir = if ($TargetDir) { [IO.Path]::GetFullPath($TargetDir).ToLowerInvariant() } else { "workspace-default" }
        command = $command
    }
    $json = $payload | ConvertTo-Json -Compress
    return [pscustomobject]@{
        fingerprint = Get-ValidationSha256 -Text $json
        payload = $payload
        snapshot = $snapshot
    }
}

Export-ModuleMember -Function Get-ValidationSha256, ConvertTo-ValidationCommand, Get-ValidationGitSnapshot, Get-ValidationFingerprint

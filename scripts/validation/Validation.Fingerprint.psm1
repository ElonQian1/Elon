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

function Initialize-ValidationCargoLock {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string[]]$CargoArgs
    )
    $manifest = $null
    for ($i = 0; $i -lt $CargoArgs.Count; $i++) {
        if ($CargoArgs[$i] -eq '--manifest-path' -and $i + 1 -lt $CargoArgs.Count) {
            $manifest = $CargoArgs[$i + 1]
            break
        }
        if ($CargoArgs[$i] -like '--manifest-path=*') {
            $manifest = $CargoArgs[$i].Substring('--manifest-path='.Length)
            break
        }
    }
    if (-not $manifest) { return }
    $manifestPath = if ([IO.Path]::IsPathRooted($manifest)) { $manifest } else { Join-Path $RepoRoot $manifest }
    $lockPath = Join-Path (Split-Path $manifestPath -Parent) 'Cargo.lock'
    if (Test-Path -LiteralPath $lockPath) { return }
    if ($CargoArgs -contains '--locked' -or $CargoArgs -contains '--frozen') { return }
    $globalArgs = New-Object System.Collections.Generic.List[string]
    for ($i = 0; $i -lt $CargoArgs.Count; $i++) {
        $arg = [string]$CargoArgs[$i]
        if ($arg -eq '--offline' -or $arg -like '--config=*') {
            $globalArgs.Add($arg)
        } elseif ($arg -eq '--config' -and $i + 1 -lt $CargoArgs.Count) {
            $globalArgs.Add($arg)
            $globalArgs.Add([string]$CargoArgs[++$i])
        }
    }
    & cargo @($globalArgs) generate-lockfile --manifest-path $manifestPath
    if ($LASTEXITCODE -ne 0) { throw "cargo generate-lockfile failed while stabilizing the validation fingerprint." }
}

function Get-ValidationGitSnapshot {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $relevantPattern = '^(server/|\.cargo/(config|config\.toml)$|rust-toolchain(\.toml)?$|rust-cache\.project\.json$|\.rustfmt-version$|scripts/(cargo-dev|validate-rust|format-rust|check-source-size)|scripts/(validation|rust-cache)/)'
    $tracked = @(& git -c core.quotepath=false -C $RepoRoot ls-files | Where-Object { $_.Replace('\','/') -match $relevantPattern })
    if ($LASTEXITCODE -ne 0) { throw "Unable to enumerate tracked validation inputs." }
    $untracked = @(& git -c core.quotepath=false -C $RepoRoot ls-files --others --exclude-standard | Where-Object { $_.Replace('\','/') -match $relevantPattern })
    $records = New-Object System.Collections.Generic.List[string]
    $all=@(@($tracked)+@($untracked)|Sort-Object -Unique); $existing=@($all|Where-Object{Test-Path -LiteralPath (Join-Path $RepoRoot $_) -PathType Leaf})
    # Windows PowerShell 5.1 prefixes native-command pipeline input with a BOM.
    # `git hash-object --stdin-paths` then treats the first tracked path as
    # "<BOM>.rustfmt-version" and fails before Cargo can run. Pass paths as
    # ordinary argv values in bounded batches so hashing is encoding-agnostic
    # without risking the Windows command-line length limit.
    $hashes = New-Object System.Collections.Generic.List[string]
    for ($offset = 0; $offset -lt $existing.Count; $offset += 64) {
        $last = [Math]::Min($offset + 63, $existing.Count - 1)
        $batch = @($existing[$offset..$last])
        $batchHashes = @(& git -C $RepoRoot hash-object --no-filters -- @batch)
        if ($LASTEXITCODE -ne 0 -or $batchHashes.Count -ne $batch.Count) {
            throw "Unable to hash validation workspace inputs."
        }
        foreach ($hash in $batchHashes) { $hashes.Add(([string]$hash).Trim()) }
    }
    if ($hashes.Count -ne $existing.Count) { throw "Unable to hash validation workspace inputs." }
    for($i=0;$i -lt $existing.Count;$i++){$records.Add("$($existing[$i].Replace('\','/'))`t$($hashes[$i].Trim())")}
    foreach($relative in @($all|Where-Object{-not (Test-Path -LiteralPath (Join-Path $RepoRoot $_) -PathType Leaf)})){$records.Add("$($relative.Replace('\','/'))`tdeleted")}
    return [pscustomobject]@{
        index = @($records)
        untracked = @($records)
        status = @()
        digest = Get-ValidationSha256 -Text ((@("--workspace-content--") + @($records)) -join "`n")
    }
}

function Get-ValidationFingerprint {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string[]]$CargoArgs,
        [string]$Domain = "dev-windows-msvc",
        [string]$TargetDir,
        $ExecutionOptions = @{}
    )
    $snapshot = Get-ValidationGitSnapshot -RepoRoot $RepoRoot
    $lockPath = Join-Path $RepoRoot "server\Cargo.lock"
    if (-not (Test-Path -LiteralPath $lockPath)) { $lockPath = Join-Path $RepoRoot "Cargo.lock" }
    $lockHash = if (Test-Path -LiteralPath $lockPath) { (& git hash-object -- $lockPath).Trim() } else { "missing" }
    $rustc = ((& rustc -vV 2>&1) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0) { throw "rustc -vV failed while creating validation fingerprint." }
    $remote = ((& git -C $RepoRoot config --get remote.origin.url 2>$null) -join "").Trim().ToLowerInvariant()
    if ([string]::IsNullOrWhiteSpace($remote)) {
        $commonDir=(((& git -C $RepoRoot rev-parse --git-common-dir 2>$null) -join '')).Trim()
        if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($commonDir)){throw 'Unable to resolve git common-dir for no-origin project identity.'}
        if(-not [IO.Path]::IsPathRooted($commonDir)){$commonDir=Join-Path $RepoRoot $commonDir}
        $remote = 'no-origin:' + (Get-ValidationSha256 ([IO.Path]::GetFullPath($commonDir).TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar).ToLowerInvariant()))
    }
    $environment = [ordered]@{}
    $names = @('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_TARGET') + @([Environment]::GetEnvironmentVariables('Process').Keys | ForEach-Object {[string]$_} | Where-Object { $_ -match '^CARGO_(PROFILE|TARGET)_.+' } | Sort-Object -Unique)
    foreach($name in ($names | Sort-Object -Unique)) { $value=[Environment]::GetEnvironmentVariable($name,'Process'); if($null -ne $value){$environment[$name]=Get-ValidationSha256 $value} }
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
        environment_hashes = $environment
        execution_options = $ExecutionOptions
    }
    $json = $payload | ConvertTo-Json -Compress
    return [pscustomobject]@{
        fingerprint = Get-ValidationSha256 -Text $json
        payload = $payload
        snapshot = $snapshot
    }
}

Export-ModuleMember -Function Get-ValidationSha256, ConvertTo-ValidationCommand, Initialize-ValidationCargoLock, Get-ValidationGitSnapshot, Get-ValidationFingerprint

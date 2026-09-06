Set-StrictMode -Version Latest

function Stop-FrameworkValidation {
    param([string]$Message)
    throw "ESK Sui Move validation failed: $Message"
}

function Get-FrameworkFileSha256 {
    param([string]$Path)
    return "sha256:$((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant())"
}

function Resolve-TrustedSystemTar {
    $systemDirectory = [Environment]::GetFolderPath('System')
    if ([string]::IsNullOrWhiteSpace($systemDirectory)) {
        Stop-FrameworkValidation "Windows system directory could not be resolved"
    }
    $tarPath = Join-Path $systemDirectory "tar.exe"
    if (-not (Test-Path -LiteralPath $tarPath -PathType Leaf)) {
        Stop-FrameworkValidation "trusted Windows tar.exe is unavailable"
    }
    $item = Get-Item -LiteralPath $tarPath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-FrameworkValidation "trusted Windows tar.exe cannot be a reparse point"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $tarPath
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
        $null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -cnotmatch '(?:^|, )O=Microsoft Corporation(?:,|$)') {
        Stop-FrameworkValidation "Windows tar.exe signature is not an approved Microsoft signature"
    }
    return $item.FullName
}

function Assert-FixedFile {
    param([string]$Path, [long]$ExpectedSize, [string]$ExpectedDigest, [string]$Label)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-FrameworkValidation "$Label is missing"
    }
    $item = Get-Item -LiteralPath $Path -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-FrameworkValidation "$Label cannot be a reparse point"
    }
    if ($item.Length -ne $ExpectedSize) {
        Stop-FrameworkValidation "$Label length differs from the fixed contract"
    }
    if ((Get-FrameworkFileSha256 -Path $item.FullName) -cne $ExpectedDigest) {
        Stop-FrameworkValidation "$Label digest differs from the fixed contract"
    }
}

function Test-ArchiveEntryName {
    param([string]$Entry, [string]$ArchiveRoot)
    if ([string]::IsNullOrWhiteSpace($Entry) -or $Entry.Contains('\') -or
        $Entry.StartsWith('/', [System.StringComparison]::Ordinal) -or
        $Entry -cmatch '^[A-Za-z]:' -or $Entry -cmatch '[\x00-\x1F\x7F]') {
        return $false
    }
    $withoutTrailingSlash = $Entry.TrimEnd('/')
    if ([string]::IsNullOrWhiteSpace($withoutTrailingSlash)) { return $false }
    $segments = $withoutTrailingSlash.Split('/')
    if ($segments | Where-Object { $_ -ceq '.' -or $_ -ceq '..' -or $_.Length -eq 0 }) {
        return $false
    }
    return $withoutTrailingSlash -ceq $ArchiveRoot -or
        $withoutTrailingSlash.StartsWith("$ArchiveRoot/", [System.StringComparison]::Ordinal)
}

function Test-TrackedTreePath {
    param([string]$RelativePath, [string[]]$TrackedRoots)
    foreach ($root in $TrackedRoots) {
        if ($RelativePath -ceq $root -or
            $RelativePath.StartsWith("$root/", [System.StringComparison]::Ordinal) -or
            $root.StartsWith("$RelativePath/", [System.StringComparison]::Ordinal)) {
            return $true
        }
    }
    return $false
}

function Get-TrackedContentDigest {
    param([string]$RepositoryRoot, [string[]]$RelativeFiles)
    $ordered = [string[]]$RelativeFiles.Clone()
    [Array]::Sort($ordered, [System.StringComparer]::Ordinal)
    $hash = [System.Security.Cryptography.IncrementalHash]::CreateHash(
        [System.Security.Cryptography.HashAlgorithmName]::SHA256)
    try {
        $zero = [byte[]]@(0)
        foreach ($relative in $ordered) {
            $hash.AppendData([System.Text.Encoding]::UTF8.GetBytes($relative))
            $hash.AppendData($zero)
            $hash.AppendData([System.IO.File]::ReadAllBytes((Join-Path $RepositoryRoot $relative)))
            $hash.AppendData($zero)
        }
        return "sha256:$([Convert]::ToHexString($hash.GetHashAndReset()).ToLowerInvariant())"
    } finally {
        $hash.Dispose()
    }
}

function Assert-FrameworkTree {
    param([string]$ExtractRoot, [object]$Contract)
    $repositoryRoot = Join-Path $ExtractRoot $Contract.framework.archive_root
    if (-not (Test-Path -LiteralPath $repositoryRoot -PathType Container)) {
        Stop-FrameworkValidation "fixed framework archive root was not extracted"
    }
    $rootItem = Get-Item -LiteralPath $repositoryRoot -Force
    if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-FrameworkValidation "fixed framework archive root cannot be a reparse point"
    }
    $trackedRoots = [string[]]@($Contract.framework.tracked_roots)
    $relativeFiles = [System.Collections.Generic.List[string]]::new()
    foreach ($item in Get-ChildItem -LiteralPath $repositoryRoot -Force -Recurse) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Stop-FrameworkValidation "fixed framework tree contains a reparse point"
        }
        $relative = [System.IO.Path]::GetRelativePath($repositoryRoot, $item.FullName).Replace('\', '/')
        if (-not (Test-TrackedTreePath -RelativePath $relative -TrackedRoots $trackedRoots)) {
            Stop-FrameworkValidation "fixed framework extraction contains a path outside the approved roots"
        }
        if (-not $item.PSIsContainer) { $relativeFiles.Add($relative) }
    }
    if ($relativeFiles.Count -ne [int]$Contract.framework.tracked_file_count) {
        Stop-FrameworkValidation "fixed framework file count differs from the fixed contract"
    }
    $digest = Get-TrackedContentDigest -RepositoryRoot $repositoryRoot -RelativeFiles $relativeFiles.ToArray()
    if ($digest -cne $Contract.framework.tracked_content_digest) {
        Stop-FrameworkValidation "fixed framework content digest differs from the fixed contract"
    }
    return Join-Path $repositoryRoot "crates/sui-framework/packages/sui-framework"
}

function Expand-FixedFrameworkArchive {
    param([string]$ArchivePath, [string]$ExtractRoot, [string]$TarPath, [object]$Contract)
    New-Item -ItemType Directory -Path $ExtractRoot -Force | Out-Null
    $entries = @(& $TarPath -tf $ArchivePath 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0 -or $entries.Count -eq 0) {
        Stop-FrameworkValidation "fixed framework archive listing failed"
    }
    foreach ($entry in $entries) {
        if (-not (Test-ArchiveEntryName -Entry $entry -ArchiveRoot $Contract.framework.archive_root)) {
            Stop-FrameworkValidation "fixed framework archive contains an unsafe entry name"
        }
    }
    foreach ($root in $Contract.framework.tracked_roots) {
        $prefix = "$($Contract.framework.archive_root)/$root/"
        if (-not ($entries | Where-Object { $_.StartsWith($prefix, [System.StringComparison]::Ordinal) } |
            Select-Object -First 1)) {
            Stop-FrameworkValidation "fixed framework archive is missing an approved root"
        }
    }
    $requestedEntries = @($Contract.framework.tracked_roots | ForEach-Object {
        "$($Contract.framework.archive_root)/$_"
    })
    $extractOutput = @(& $TarPath -xf $ArchivePath -C $ExtractRoot @requestedEntries 2>&1 |
        ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0 -or ($extractOutput | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })) {
        Stop-FrameworkValidation "fixed framework archive extraction failed"
    }
    return Assert-FrameworkTree -ExtractRoot $ExtractRoot -Contract $Contract
}

Export-ModuleMember -Function @(
    'Assert-FixedFile',
    'Assert-FrameworkTree',
    'Expand-FixedFrameworkArchive',
    'Resolve-TrustedSystemTar'
)

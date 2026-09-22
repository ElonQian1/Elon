function Assert-ElonApkWebChatAssets {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$ApkPath
    )

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $assetRoot = Join-Path $RepoRoot 'android/app/src/main/assets'
    $sources = @(Get-ChildItem -LiteralPath $assetRoot -Filter 'chatgpt_web*.js' -File -ErrorAction Stop)
    if ($sources.Count -eq 0) { throw 'APK_SOURCE_ASSETS_UNAVAILABLE' }
    $archive = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $ApkPath).Path)
    try {
        foreach ($source in $sources) {
            $name = 'assets/' + $source.Name
            $entries = @($archive.Entries | Where-Object { $_.FullName -ceq $name })
            if ($entries.Count -ne 1) { throw "APK_SOURCE_ASSET_MISSING_OR_DUPLICATE: $($source.Name)" }
            $expected = (Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash
            $stream = $entries[0].Open()
            $sha = [System.Security.Cryptography.SHA256]::Create()
            try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
            finally { $sha.Dispose(); $stream.Dispose() }
            if ($actual -ne $expected) { throw "APK_SOURCE_ASSET_MISMATCH: $($source.Name)" }
        }
        Write-Host "APK_SOURCE_ASSETS=verified count=$($sources.Count)"
    } finally { $archive.Dispose() }
}

function Assert-ApkManifestVersion {
    param(
        [Parameter(Mandatory)][string]$ApkPath,
        [Parameter(Mandatory)][int]$ExpectedVersionCode,
        [Parameter(Mandatory)][string]$ExpectedVersionName,
        [Parameter(Mandatory)][string]$SourceRoot,
        [string]$Label = 'APK'
    )

    $actual = Get-ApkManifestVersion -ApkPath $ApkPath
    if ($actual.VersionCode -ne $ExpectedVersionCode -or $actual.VersionName -ne $ExpectedVersionName) {
        throw "$Label manifest mismatch: expected $ExpectedVersionName ($ExpectedVersionCode), got $($actual.VersionName) ($($actual.VersionCode))."
    }
    Assert-ElonApkWebChatAssets -RepoRoot $SourceRoot -ApkPath $ApkPath
    Write-Host "APK_MANIFEST=verified version=$($actual.VersionName) code=$($actual.VersionCode)"
}

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'apk-publish-artifact-validation.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$root = Join-Path $tempRoot ('elon-apk-assets-' + [Guid]::NewGuid().ToString('N'))
$assets = Join-Path $root 'android/app/src/main/assets'
$null = New-Item -ItemType Directory -Path $assets -Force
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Write-FixtureApk {
    param([string]$Name, [string[]]$Contents)
    $path = Join-Path $root ($Name + '.apk')
    $archive = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($content in $Contents) {
            $entry = $archive.CreateEntry('assets/chatgpt_web_fixture.js')
            $stream = $entry.Open()
            try { $bytes = $utf8.GetBytes($content); $stream.Write($bytes, 0, $bytes.Length) }
            finally { $stream.Dispose() }
        }
    } finally { $archive.Dispose() }
    return $path
}

function Assert-Fails {
    param([scriptblock]$Action, [string]$Expected)
    $message = ''
    try { & $Action } catch { $message = $_.Exception.Message }
    if ($message -notlike "*$Expected*") { throw "Expected $Expected; got $message" }
}

try {
    [System.IO.File]::WriteAllText((Join-Path $assets 'chatgpt_web_fixture.js'), 'version2', $utf8)
    $good = Write-FixtureApk 'good' @('version2')
    Assert-ElonApkWebChatAssets -RepoRoot $root -ApkPath $good
    $old = Write-FixtureApk 'old' @('version1')
    Assert-Fails { Assert-ElonApkWebChatAssets -RepoRoot $root -ApkPath $old } 'APK_SOURCE_ASSET_MISMATCH'
    $missing = Write-FixtureApk 'missing' @()
    Assert-Fails { Assert-ElonApkWebChatAssets -RepoRoot $root -ApkPath $missing } 'APK_SOURCE_ASSET_MISSING_OR_DUPLICATE'
    $duplicate = Write-FixtureApk 'duplicate' @('version2', 'version2')
    Assert-Fails { Assert-ElonApkWebChatAssets -RepoRoot $root -ApkPath $duplicate } 'APK_SOURCE_ASSET_MISSING_OR_DUPLICATE'
    function Get-ApkManifestVersion { return @{ VersionCode = 1; VersionName = '1.0' } }
    Assert-Fails { Assert-ApkManifestVersion -ApkPath $good -ExpectedVersionCode 2 -ExpectedVersionName '1.0' -SourceRoot $root } 'manifest mismatch'
    Assert-Fails { Assert-ApkManifestVersion -ApkPath $old -ExpectedVersionCode 1 -ExpectedVersionName '1.0' -SourceRoot $root } 'APK_SOURCE_ASSET_MISMATCH'
    Assert-ApkManifestVersion -ApkPath $good -ExpectedVersionCode 1 -ExpectedVersionName '1.0' -SourceRoot $root
    Write-Host 'APK_ARTIFACT_VALIDATION_TEST=passed cases=7'
} finally {
    $resolved = [System.IO.Path]::GetFullPath($root)
    if ($resolved.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
        [System.IO.Path]::GetFileName($resolved).StartsWith('elon-apk-assets-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    } else { throw 'Fixture cleanup target is outside the temporary workspace' }
}

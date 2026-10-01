[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('Host','Client')][string]$Role,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSourceSha256,
    [string]$HostName = ((-join [char[]](0x5fd7,0x4f1f)) + '4060'),
    [string]$ShareName = (-join [char[]](0x4e00,0x9f99,0x8d44,0x6599)),
    [string]$CacheDirectory = (-join [char[]](0x4e00,0x9f99,0x7f13,0x5b58)),
    [string]$LocalControlRoot,
    [string]$ProjectRoot,
    [switch]$AllowUncBuild,
    [switch]$Apply
)
$ErrorActionPreference = 'Stop'
function Get-StudioBootstrapBundleHash {
    param([string]$Path)
    $stream=[IO.File]::OpenRead($Path); $sha=[Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() } finally { $sha.Dispose();$stream.Dispose() }
}
$sourceRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
# Validate the reviewed bundle before importing any of its implementation modules.
if ($sourceRoot -notmatch '^[A-Za-z]:\\' -or ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($sourceRoot))).DriveType -ne 'Fixed') {
    throw 'STUDIO_BUNDLE_LOCAL_REQUIRED: Extract the reviewed bundle to a local fixed disk first.'
}
$ancestor = [IO.DirectoryInfo]::new($sourceRoot)
while ($ancestor) {
    if ((Get-Item -LiteralPath $ancestor.FullName -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'STUDIO_BUNDLE_REPARSE: Linked source roots are not allowed.' }
    $ancestor = $ancestor.Parent
}
$manifestPath = Join-Path $sourceRoot 'studio-cache-bundle.json'
if ((Get-Item -LiteralPath $manifestPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'STUDIO_BUNDLE_REPARSE: Linked manifest.' }
if ((Get-StudioBootstrapBundleHash $manifestPath) -ne $ExpectedSourceSha256) { throw 'STUDIO_BUNDLE_HASH: Reviewed bundle digest does not match.' }
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.schema -ne 'elon.studio_cache.bundle.v1' -or @($manifest.files).Count -eq 0) { throw 'STUDIO_BUNDLE_SCHEMA: Invalid bundle manifest.' }
$expected = @{}
foreach ($file in $manifest.files) {
    $relative = [string]$file.path
    if ($relative -match '(^/|\\|:|(^|/)\.\.?(/|$))' -or $relative -match '[*?]' -or [string]::IsNullOrWhiteSpace($relative) -or $file.sha256 -notmatch '^[a-fA-F0-9]{64}$' -or $expected.ContainsKey($relative)) { throw 'STUDIO_BUNDLE_PATH: Invalid or duplicate manifest path.' }
    $expected[$relative] = [string]$file.sha256
}
$pending = New-Object 'Collections.Generic.Stack[string]'
$pending.Push($sourceRoot)
$seen = 0
while ($pending.Count) {
    foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'STUDIO_BUNDLE_REPARSE: Linked bundle content.' }
        if ($item.PSIsContainer) { $pending.Push($item.FullName); continue }
        if ($item.FullName -eq $manifestPath) { continue }
        $relative = $item.FullName.Substring($sourceRoot.Length + 1).Replace('\','/')
        if (-not $expected.ContainsKey($relative) -or (Get-StudioBootstrapBundleHash $item.FullName) -ne $expected[$relative]) { throw "STUDIO_BUNDLE_CONTENT: Unreviewed or changed file: $relative" }
        $seen++
    }
}
if ($seen -ne $expected.Count) { throw 'STUDIO_BUNDLE_CONTENT: Reviewed bundle files are missing.' }
foreach ($required in @('scripts/studio-cache/StudioCache.Bootstrap.psm1','scripts/rust-cache.ps1','rust-cache.project.json')) {
    if (-not $expected.ContainsKey($required)) { throw "STUDIO_BUNDLE_CONTENT: Required file missing: $required" }
}
Import-Module (Join-Path $PSScriptRoot 'studio-cache\StudioCache.Bootstrap.psm1') -Force -DisableNameChecking
$arguments = @{ Role=$Role; HostName=$HostName; ShareName=$ShareName; CacheDirectory=$CacheDirectory; SourceRoot=$sourceRoot; SourceSha256=$ExpectedSourceSha256; AllowUncBuild=$AllowUncBuild; Apply=$Apply }
if ($LocalControlRoot) { $arguments.LocalControlRoot = $LocalControlRoot }
if ($ProjectRoot) { $arguments.ProjectRoot = $ProjectRoot }
Invoke-StudioCacheBootstrap @arguments | ConvertTo-Json -Depth 12

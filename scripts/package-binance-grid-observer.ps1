param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$observerRepo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$observerSource = Join-Path $observerRepo 'tools\binance-grid-observer'
$observerOutput = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $observerOutput) { throw 'OutputDirectory must be new; existing artifacts are preserved.' }
$observerFiles = @('manifest.json', 'sanitize.js', 'observer.js', 'bridge.js', 'store.js', 'worker.js',
    'popup.html', 'popup.css', 'popup.js', 'README.md')
foreach ($observerFile in $observerFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $observerSource $observerFile) -PathType Leaf)) {
        throw "Missing extension source: $observerFile"
    }
}
$observerChanges = @(git -C $observerRepo status --porcelain -- tools/binance-grid-observer scripts/package-binance-grid-observer.ps1)
if ($LASTEXITCODE -ne 0 -or $observerChanges.Count -gt 0) { throw 'Commit extension and packaging sources before creating an identified artifact.' }
$observerCommit = (git -C $observerRepo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $observerCommit -notmatch '^[0-9a-f]{40}$') { throw 'Source commit unavailable.' }
$observerManifest = Get-Content -LiteralPath (Join-Path $observerSource 'manifest.json') -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Path $observerOutput | Out-Null
$observerExtension = Join-Path $observerOutput 'extension'
New-Item -ItemType Directory -Path $observerExtension | Out-Null
$observerHashes = @()
foreach ($observerFile in $observerFiles) {
    $observerDestination = Join-Path $observerExtension $observerFile
    Copy-Item -LiteralPath (Join-Path $observerSource $observerFile) -Destination $observerDestination
    $observerHashes += @{ path = $observerFile; sha256 = (Get-FileHash -LiteralPath $observerDestination -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$observerIdentity = @{ schema = 'binance-grid-observer-artifact.v1'; source_commit = $observerCommit;
    version = $observerManifest.version; files = $observerHashes; live_browser_verified = $false }
$observerEncoding = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText((Join-Path $observerOutput 'artifact.json'), ($observerIdentity | ConvertTo-Json -Depth 6), $observerEncoding)
$observerZip = Join-Path $observerOutput ('binance-grid-observer-' + $observerManifest.version + '.zip')
Compress-Archive -Path (Join-Path $observerExtension '*') -DestinationPath $observerZip -CompressionLevel Optimal
$observerZipHash = (Get-FileHash -LiteralPath $observerZip -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText(($observerZip + '.sha256'), ($observerZipHash + '  ' + [System.IO.Path]::GetFileName($observerZip)), $observerEncoding)
Write-Output 'ARTIFACT_READY=true'
Write-Output "SOURCE_COMMIT=$observerCommit"
Write-Output "EXTENSION_DIRECTORY=$observerExtension"
Write-Output "ARTIFACT_ZIP=$observerZip"
Write-Output "ARTIFACT_SHA256=$observerZipHash"
Write-Output 'LIVE_BROWSER_VERIFIED=false'

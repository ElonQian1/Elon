param(
    [Parameter(Mandatory = $true)]
    [string]$ZipPath
)

$ErrorActionPreference = 'Stop'
$resolvedZip = (Resolve-Path -LiteralPath $ZipPath).Path
if ([IO.Path]::GetExtension($resolvedZip) -ne '.zip') {
    throw 'STITCH_EXPORT_INVALID=expected_zip'
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($resolvedZip)
try {
    $entries = @{}
    foreach ($entry in $archive.Entries) {
        $leaf = [IO.Path]::GetFileName($entry.FullName).ToLowerInvariant()
        if ($leaf -and -not $entries.ContainsKey($leaf)) {
            $entries[$leaf] = $entry
        }
    }

    function Get-EntrySha256([IO.Compression.ZipArchiveEntry]$Entry) {
        $stream = $Entry.Open()
        try {
            $sha = [Security.Cryptography.SHA256]::Create()
            try { return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
            finally { $sha.Dispose() }
        } finally { $stream.Dispose() }
    }

    function Get-PngDimensions([IO.Compression.ZipArchiveEntry]$Entry) {
        if ($null -eq $Entry) { return $null }
        $stream = $Entry.Open()
        try {
            $header = New-Object byte[] 24
            if ($stream.Read($header, 0, 24) -ne 24) { throw 'STITCH_EXPORT_INVALID=truncated_screen_png' }
            $signature = '89504E470D0A1A0A'
            if (([BitConverter]::ToString($header[0..7])).Replace('-', '') -ne $signature) {
                throw 'STITCH_EXPORT_INVALID=screen_not_png'
            }
            $width = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($header, 16))
            $height = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($header, 20))
            return [ordered]@{ width = $width; height = $height }
        } finally { $stream.Dispose() }
    }

    $required = @('code.html', 'design.md', 'screen.png')
    $files = foreach ($name in $required) {
        $entry = $entries[$name]
        [ordered]@{
            name = $name
            present = $null -ne $entry
            bytes = if ($entry) { $entry.Length } else { 0 }
            sha256 = if ($entry) { Get-EntrySha256 $entry } else { $null }
        }
    }
    $presentCount = @($files | Where-Object present).Count
    $quality = if ($presentCount -eq 3) { 'FULL' } elseif ($entries.ContainsKey('screen.png')) { 'PARTIAL' } else { 'INSUFFICIENT' }
    $zipHash = (Get-FileHash -LiteralPath $resolvedZip -Algorithm SHA256).Hash.ToLowerInvariant()

    [ordered]@{
        schema = 'elon.stitch_export_inspection.v1'
        quality = $quality
        zipSha256 = $zipHash
        files = $files
        designViewport = Get-PngDimensions $entries['screen.png']
        implementationPolicy = if ($quality -eq 'FULL') { 'structured_parameters_available' } else { 'missing_values_must_be_marked_inferred' }
    } | ConvertTo-Json -Depth 5

    if ($quality -eq 'INSUFFICIENT') { exit 2 }
} finally {
    $archive.Dispose()
}

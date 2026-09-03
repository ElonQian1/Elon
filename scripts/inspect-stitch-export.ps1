param(
    [Parameter(Mandatory = $true)]
    [string]$ZipPath,
    [string]$OutputPath = "",
    [switch]$RequireFull
)

$ErrorActionPreference = 'Stop'
$required = @('code.html', 'design.md', 'screen.png')
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
        if (-not $leaf) { continue }
        if ($required -contains $leaf -and $entries.ContainsKey($leaf)) {
            throw "STITCH_EXPORT_INVALID=duplicate_required_file:$leaf"
        }
        if (-not $entries.ContainsKey($leaf)) {
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
            if (([BitConverter]::ToString($header[0..7])).Replace('-', '') -ne '89504E470D0A1A0A') {
                throw 'STITCH_EXPORT_INVALID=screen_not_png'
            }
            if ([Text.Encoding]::ASCII.GetString($header, 12, 4) -ne 'IHDR') {
                throw 'STITCH_EXPORT_INVALID=screen_missing_ihdr'
            }
            $width = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($header, 16))
            $height = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($header, 20))
            if ($width -le 0 -or $height -le 0) { throw 'STITCH_EXPORT_INVALID=screen_dimensions' }
            return [ordered]@{ width = $width; height = $height }
        } finally { $stream.Dispose() }
    }

    $files = foreach ($name in $required) {
        $entry = $entries[$name]
        [ordered]@{
            name = $name
            present = $null -ne $entry
            bytes = if ($entry) { $entry.Length } else { 0 }
            sha256 = if ($entry) { Get-EntrySha256 $entry } else { $null }
        }
    }
    $missingRequiredFiles = @($files | Where-Object { -not $_.present } | ForEach-Object { $_.name })
    $quality = if ($missingRequiredFiles.Count -eq 0) {
        'FULL'
    } elseif ($entries.ContainsKey('screen.png')) {
        'PARTIAL'
    } else {
        'INSUFFICIENT'
    }
    $evidenceKinds = @()
    if ($entries.ContainsKey('code.html')) { $evidenceKinds += 'EXACT_CODE' }
    if ($entries.ContainsKey('design.md')) { $evidenceKinds += 'EXACT_DESIGN' }
    if ($entries.ContainsKey('screen.png')) { $evidenceKinds += 'MEASURED_IMAGE' }

    $result = [ordered]@{
        schema = 'elon.stitch_export_inspection.v1'
        quality = $quality
        zipSha256 = (Get-FileHash -LiteralPath $resolvedZip -Algorithm SHA256).Hash.ToLowerInvariant()
        files = $files
        missingRequiredFiles = $missingRequiredFiles
        designViewport = Get-PngDimensions $entries['screen.png']
        evidenceKinds = $evidenceKinds
        implementationPolicy = if ($quality -eq 'FULL') {
            'structured_parameters_available'
        } else {
            'missing_values_must_be_marked_inferred'
        }
        claimPolicy = [ordered]@{
            exportEvidenceGate = if ($quality -eq 'FULL') { 'PASSED' } else { 'BLOCKED' }
            exactParameterSourceAvailable = $quality -eq 'FULL'
            oneToOneClaimFromExportAlone = $false
            remainingPreconditions = @(
                'no_inferred_critical_parameters',
                'same_viewport_runtime_capture',
                'visual_comparison_within_project_threshold'
            )
        }
    }
    $json = $result | ConvertTo-Json -Depth 6

    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $resolvedOutput = if ([IO.Path]::IsPathRooted($OutputPath)) {
            [IO.Path]::GetFullPath($OutputPath)
        } else {
            [IO.Path]::GetFullPath((Join-Path (Get-Location).Path $OutputPath))
        }
        $outputDirectory = [IO.Path]::GetDirectoryName($resolvedOutput)
        [IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
        $temporaryOutput = Join-Path $outputDirectory ('.{0}.{1}.tmp' -f [IO.Path]::GetFileName($resolvedOutput), [Guid]::NewGuid().ToString('N'))
        try {
            [IO.File]::WriteAllText($temporaryOutput, $json, (New-Object Text.UTF8Encoding($false)))
            Move-Item -LiteralPath $temporaryOutput -Destination $resolvedOutput -Force
        } finally {
            if (Test-Path -LiteralPath $temporaryOutput) {
                Remove-Item -LiteralPath $temporaryOutput -Force
            }
        }
    }

    Write-Output $json
    if ($quality -eq 'INSUFFICIENT') { exit 2 }
    if ($RequireFull -and $quality -ne 'FULL') { exit 3 }
} finally {
    $archive.Dispose()
}

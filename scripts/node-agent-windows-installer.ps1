Set-StrictMode -Version Latest

$script:NodeAgentInstallerMagic = [System.Text.Encoding]::ASCII.GetBytes(
    'ELON_NODE_INSTALLER_PAYLOAD_V1!!'
)
$script:NodeAgentInstallerFooterSize = $script:NodeAgentInstallerMagic.Length + 8 + 32
$script:NodeAgentInstallerSearchLimit = 4MB

function Find-NodeAgentWindowsInstallerPayload {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Installer does not exist: $Path"
    }
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $fileLength = $stream.Length
        if ($fileLength -lt $script:NodeAgentInstallerFooterSize) {
            throw 'Installer is too short to contain a payload footer.'
        }
        $tailLength = [int][Math]::Min(
            [int64]$script:NodeAgentInstallerSearchLimit,
            [int64]$fileLength
        )
        $tail = New-Object byte[] $tailLength
        $stream.Seek(-$tailLength, [System.IO.SeekOrigin]::End) | Out-Null
        $offset = 0
        while ($offset -lt $tail.Length) {
            $read = $stream.Read($tail, $offset, $tail.Length - $offset)
            if ($read -le 0) { throw 'Installer tail ended unexpectedly.' }
            $offset += $read
        }
        for ($index = $tail.Length - $script:NodeAgentInstallerFooterSize; $index -ge 0; $index--) {
            $matched = $true
            for ($magicIndex = 0; $magicIndex -lt $script:NodeAgentInstallerMagic.Length; $magicIndex++) {
                if ($tail[$index + $magicIndex] -ne $script:NodeAgentInstallerMagic[$magicIndex]) {
                    $matched = $false
                    break
                }
            }
            if (-not $matched) { continue }
            $lengthOffset = $index + $script:NodeAgentInstallerMagic.Length
            $payloadLength = [System.BitConverter]::ToUInt64($tail, $lengthOffset)
            $footerOffset = [uint64]($fileLength - $tailLength + $index)
            if ($payloadLength -eq 0 -or $payloadLength -gt $footerOffset) { continue }
            $shaOffset = $lengthOffset + 8
            $sha = New-Object byte[] 32
            [Array]::Copy($tail, $shaOffset, $sha, 0, 32)
            return [pscustomobject]@{
                PayloadOffset = [uint64]($footerOffset - $payloadLength)
                PayloadLength = [uint64]$payloadLength
                PayloadSha256 = (
                    [System.BitConverter]::ToString($sha).Replace('-', '').ToLowerInvariant()
                )
                FooterOffset = $footerOffset
                TrailingByteCount = [uint64]($fileLength - $footerOffset -
                    $script:NodeAgentInstallerFooterSize)
            }
        }
        throw 'Installer payload footer was not found.'
    } finally {
        $stream.Dispose()
    }
}

function Get-NodeAgentInstallerRangeSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][uint64]$Offset,
        [Parameter(Mandatory = $true)][uint64]$Length
    )
    $stream = [System.IO.File]::OpenRead($Path)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream.Seek([int64]$Offset, [System.IO.SeekOrigin]::Begin) | Out-Null
        $remaining = $Length
        $buffer = New-Object byte[] (1024 * 1024)
        while ($remaining -gt 0) {
            $requested = [int][Math]::Min([uint64]$buffer.Length, $remaining)
            $read = $stream.Read($buffer, 0, $requested)
            if ($read -le 0) { throw 'Installer payload ended unexpectedly.' }
            $sha256.TransformBlock($buffer, 0, $read, $buffer, 0) | Out-Null
            $remaining -= [uint64]$read
        }
        $sha256.TransformFinalBlock((New-Object byte[] 0), 0, 0) | Out-Null
        return [System.BitConverter]::ToString($sha256.Hash).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha256.Dispose()
        $stream.Dispose()
    }
}

function Test-NodeAgentWindowsInstallerPackage {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$ExpectedPayloadSha256 = ''
    )
    $descriptor = Find-NodeAgentWindowsInstallerPayload -Path $Path
    $actual = Get-NodeAgentInstallerRangeSha256 -Path $Path `
        -Offset $descriptor.PayloadOffset -Length $descriptor.PayloadLength
    if ($actual -ne $descriptor.PayloadSha256) {
        throw 'Installer embedded payload SHA-256 does not match its footer.'
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedPayloadSha256) -and
        $actual -ne $ExpectedPayloadSha256.Trim().ToLowerInvariant()) {
        throw 'Installer embedded payload differs from the expected Windows client package.'
    }
    [pscustomobject]@{
        Path = [System.IO.Path]::GetFullPath($Path)
        FileSize = (Get-Item -LiteralPath $Path).Length
        PayloadOffset = $descriptor.PayloadOffset
        PayloadLength = $descriptor.PayloadLength
        PayloadSha256 = $actual
        TrailingByteCount = $descriptor.TrailingByteCount
    }
}

function New-NodeAgentWindowsInstallerPackage {
    param(
        [Parameter(Mandatory = $true)][string]$StubPath,
        [Parameter(Mandatory = $true)][string]$PayloadPath,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )
    foreach ($inputPath in @($StubPath, $PayloadPath)) {
        if (-not (Test-Path -LiteralPath $inputPath -PathType Leaf)) {
            throw "Installer input does not exist: $inputPath"
        }
    }
    $parent = Split-Path -Parent $OutputPath
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    Copy-Item -LiteralPath $StubPath -Destination $OutputPath -Force

    $payloadSha256 = Get-NodeAgentFileSha256 -Path $PayloadPath
    $payload = [System.IO.File]::OpenRead($PayloadPath)
    $output = [System.IO.File]::Open(
        $OutputPath,
        [System.IO.FileMode]::Append,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $payload.CopyTo($output)
        $output.Write(
            $script:NodeAgentInstallerMagic,
            0,
            $script:NodeAgentInstallerMagic.Length
        )
        $lengthBytes = [System.BitConverter]::GetBytes([uint64]$payload.Length)
        $output.Write($lengthBytes, 0, $lengthBytes.Length)
        $shaBytes = New-Object byte[] 32
        for ($index = 0; $index -lt 32; $index++) {
            $shaBytes[$index] = [Convert]::ToByte($payloadSha256.Substring($index * 2, 2), 16)
        }
        $output.Write($shaBytes, 0, $shaBytes.Length)
        $output.Flush()
    } finally {
        $output.Dispose()
        $payload.Dispose()
    }
    Test-NodeAgentWindowsInstallerPackage -Path $OutputPath `
        -ExpectedPayloadSha256 $payloadSha256
}

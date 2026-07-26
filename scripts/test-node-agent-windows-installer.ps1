$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
. (Join-Path $PSScriptRoot 'node-agent-windows-installer.ps1')

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) (
    'elon-node-installer-test-' + [Guid]::NewGuid().ToString('N')
)
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $stub = Join-Path $root 'stub.exe'
    $payload = Join-Path $root 'client.zip'
    $installer = Join-Path $root 'Elon-Windows-Setup.exe'
    [System.IO.File]::WriteAllBytes($stub, [byte[]](77, 90, 1, 2, 3, 4))
    [System.IO.File]::WriteAllBytes($payload, [byte[]](80, 75, 5, 6, 7, 8, 9))
    $payloadSha = Get-NodeAgentFileSha256 -Path $payload

    $created = New-NodeAgentWindowsInstallerPackage -StubPath $stub `
        -PayloadPath $payload -OutputPath $installer
    Assert-True ($created.PayloadSha256 -eq $payloadSha) `
        'created installer must bind the exact client package SHA-256'
    Assert-True ($created.PayloadOffset -eq (Get-Item -LiteralPath $stub).Length) `
        'payload must start immediately after the executable stub'
    Assert-True ($created.TrailingByteCount -eq 0) `
        'unsigned fixture must end at the payload footer'

    $trailer = [System.Text.Encoding]::ASCII.GetBytes('fake-authenticode-trailer')
    $stream = [System.IO.File]::Open(
        $installer,
        [System.IO.FileMode]::Append,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $stream.Write($trailer, 0, $trailer.Length)
    } finally {
        $stream.Dispose()
    }
    $signedShape = Test-NodeAgentWindowsInstallerPackage -Path $installer `
        -ExpectedPayloadSha256 $payloadSha
    Assert-True ($signedShape.TrailingByteCount -eq $trailer.Length) `
        'payload validation must tolerate an Authenticode trailer appended after packaging'

    [System.IO.File]::WriteAllBytes($payload, [byte[]](1, 1, 2, 3, 5, 8))
    $mismatchRejected = $false
    try {
        Test-NodeAgentWindowsInstallerPackage -Path $installer `
            -ExpectedPayloadSha256 (Get-NodeAgentFileSha256 -Path $payload) | Out-Null
    } catch {
        $mismatchRejected = $_.Exception.Message.Contains('differs from the expected')
    }
    Assert-True $mismatchRejected 'stale installer/client combinations must fail closed'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'NODE_AGENT_WINDOWS_INSTALLER_TESTS=passed' -ForegroundColor Green

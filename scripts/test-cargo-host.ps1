$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'validation\Cargo.Network.psm1') -Force -DisableNameChecking
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\.ai-tmp'))
$fixture = [IO.Path]::GetFullPath((Join-Path $root ('cargo-host-' + [Guid]::NewGuid().ToString('N'))))
if (-not $fixture.StartsWith($root + [IO.Path]::DirectorySeparatorChar)) { throw 'Fixture escaped task temporary directory' }
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
try {
    $scriptPath = Join-Path $fixture 'cargo-probe.ps1'
    @'
$ErrorActionPreference = 'Stop'
Add-Type 'public class CargoHostProbe { public static int Value = 7; }'
Write-Output "HOST_MAJOR=$($PSVersionTable.PSVersion.Major)"
Write-Output "PROBE=$([CargoHostProbe]::Value)"
'@ | Set-Content -LiteralPath $scriptPath -Encoding UTF8
    $result = Invoke-CargoManagedAttempt -RepoRoot (Split-Path $PSScriptRoot -Parent) -CargoDevPath $scriptPath -EvidenceDirectory (Join-Path $fixture 'evidence') -CargoArguments @('check') -TimeoutSeconds 30
    if ($result.exit_code -ne 0) { throw "Child failed: $(Get-Content -LiteralPath $result.stderr_path -Raw)" }
    $output = Get-Content -LiteralPath $result.stdout_path -Raw
    if ($output -notmatch "HOST_MAJOR=$($PSVersionTable.PSVersion.Major)" -or $output -notmatch 'PROBE=7') { throw "Child runtime mismatch: $output" }
    Write-Output "CARGO_HOST_TEST=passed major=$($PSVersionTable.PSVersion.Major)"
} finally {
    if ([IO.Path]::GetFullPath($fixture).StartsWith($root + [IO.Path]::DirectorySeparatorChar)) { Remove-Item -LiteralPath $fixture -Recurse -Force }
}

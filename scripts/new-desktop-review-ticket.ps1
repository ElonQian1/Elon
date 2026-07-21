param(
  [Parameter(Mandatory=$true)][string]$OwnerUserId,
  [Parameter(Mandatory=$true)][string]$TaskId,
  [Parameter(Mandatory=$true)][string]$Method,
  [Parameter(Mandatory=$true)][string]$EndpointPath,
  [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$BodySha256,
  [Parameter(Mandatory=$true)][string]$StateRoot,
  [Parameter(Mandatory=$true)][string]$InstallRoot
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$StateRoot = [IO.Path]::GetFullPath($StateRoot)
$InstallRoot = [IO.Path]::GetFullPath($InstallRoot)
$statePath = Join-Path $StateRoot 'active.json'
if (-not (Test-Path -LiteralPath $statePath)) { throw 'desktop review signing capability is not configured' }
$state = Get-Content -Raw -LiteralPath $statePath | ConvertFrom-Json
if ($state.schema -notin @(2,3) -or [IO.Path]::GetFullPath([string]$state.state_root) -ne $StateRoot -or
    [IO.Path]::GetFullPath([string]$state.install_root) -ne $InstallRoot) {
  throw 'fail_closed: signing state does not match explicit StateRoot and InstallRoot'
}
$currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
if ($currentSid -ne $state.desktop_sid -or $currentSid -eq $state.executor_sid) {
  throw 'fail_closed: process identity is not the configured Desktop identity'
}
$cert = Get-Item ([string]$state.certificate_store + '\' + $state.thumbprint)
$rsa = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
if ($null -eq $rsa) { throw 'desktop review signing key is unavailable to this process identity' }
$expires = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds() + 120
$nonceBytes = New-Object byte[] 24
[Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($nonceBytes)
$nonce = ([Convert]::ToBase64String($nonceBytes)).TrimEnd('=').Replace('+','-').Replace('/','-')
function Field([string]$Value) { ([Text.Encoding]::UTF8.GetByteCount($Value)).ToString() + ':' + $Value }
$methodValue = $Method.ToUpperInvariant()
if (-not $EndpointPath.StartsWith('/') -or $EndpointPath.Contains('?') -or $EndpointPath.Contains('#')) {
  throw 'fail_closed: EndpointPath must be an absolute canonical path without query or fragment'
}
$message = @('v3',(Field $OwnerUserId),(Field $TaskId),(Field $methodValue),(Field $EndpointPath),
  (Field $BodySha256.ToLowerInvariant()),$expires.ToString(),(Field $nonce),(Field ([string]$state.key_id))) -join "`n"
$signature = $rsa.SignData([Text.Encoding]::UTF8.GetBytes($message), [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
'v3.{0}.{1}.{2}.{3}' -f $state.key_id,$expires,$nonce,[Convert]::ToBase64String($signature)

param(
  [Parameter(Mandatory=$true)][string]$OwnerUserId,
  [Parameter(Mandatory=$true)][string]$TaskId,
  [string]$StatePath = (Join-Path $env:LOCALAPPDATA 'ElonNode\_internal\desktop-review-auth\active.json')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
if (-not (Test-Path -LiteralPath $StatePath)) { throw 'desktop review signing capability is not configured' }
$state = Get-Content -Raw -LiteralPath $StatePath | ConvertFrom-Json
$cert = Get-Item ('Cert:\CurrentUser\My\' + $state.thumbprint)
$rsa = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
if ($null -eq $rsa) { throw 'desktop review signing key is unavailable to this process identity' }
$expires = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds() + 120
$nonceBytes = New-Object byte[] 24
[Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($nonceBytes)
$nonce = ([Convert]::ToBase64String($nonceBytes)).TrimEnd('=').Replace('+','-').Replace('/','-')
$message = "v2`n$OwnerUserId`n$TaskId`n$expires`n$nonce"
$signature = $rsa.SignData([Text.Encoding]::UTF8.GetBytes($message), [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
'v2.{0}.{1}.{2}.{3}' -f $state.key_id,$expires,$nonce,[Convert]::ToBase64String($signature)

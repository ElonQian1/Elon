[CmdletBinding(SupportsShouldProcess = $true)]
param(
  [ValidateSet('Prepare','Validate','Commit','Diagnose','Rollback')][string]$Action = 'Diagnose',
  [string]$DesktopIdentitySid,
  [string]$ExecutorIdentitySid,
  [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'ElonNode'),
  [switch]$Rotate
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$stateRoot = Join-Path $env:LOCALAPPDATA 'ElonNode\_internal\desktop-review-auth'
$stagePath = Join-Path $stateRoot 'prepared.json'
$activePath = Join-Path $stateRoot 'active.json'
$backupPath = Join-Path $stateRoot 'rollback.json'
$envPath = Join-Path $InstallRoot '_internal\node-agent.env'

function Write-Result([string]$Status, [string]$Detail) {
  [pscustomobject]@{ action=$Action; status=$Status; detail=$Detail; secret_exposed=$false } |
    ConvertTo-Json -Compress | Write-Output
}
function Assert-SeparateIdentities {
  if ([string]::IsNullOrWhiteSpace($DesktopIdentitySid) -or
      [string]::IsNullOrWhiteSpace($ExecutorIdentitySid) -or
      $DesktopIdentitySid -eq $ExecutorIdentitySid) {
    throw 'fail_closed: DesktopIdentitySid and ExecutorIdentitySid must be present and different'
  }
  $null = New-Object Security.Principal.SecurityIdentifier($DesktopIdentitySid)
  $null = New-Object Security.Principal.SecurityIdentifier($ExecutorIdentitySid)
}
function Get-KeyFile([Security.Cryptography.X509Certificates.X509Certificate2]$Cert) {
  $rsa = $Cert.GetRSAPrivateKey()
  if ($null -eq $rsa -or $null -eq $rsa.Key) { throw 'CNG private key is unavailable' }
  $name = $rsa.Key.UniqueName
  Join-Path $env:APPDATA ('Microsoft\Crypto\Keys\' + $name)
}
function Public-Record([Security.Cryptography.X509Certificates.X509Certificate2]$Cert) {
  $rsa = $Cert.GetRSAPublicKey()
  $p = $rsa.ExportParameters($false)
  $fingerprint = [Security.Cryptography.SHA256]::Create().ComputeHash($p.Modulus)
  $keyId = -join ($fingerprint[0..7] | ForEach-Object { $_.ToString('x2') })
  [pscustomobject]@{
    schema=1; key_id=$keyId; thumbprint=$Cert.Thumbprint;
    public_value=($keyId + ':' + [Convert]::ToBase64String($p.Modulus) + ':' + [Convert]::ToBase64String($p.Exponent));
    key_file=(Get-KeyFile $Cert); desktop_sid=$DesktopIdentitySid; executor_sid=$ExecutorIdentitySid
  }
}
function Read-State([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return $null }
  Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}
function Set-PublicValues([string[]]$Values) {
  $lines = if (Test-Path -LiteralPath $envPath) { @(Get-Content -LiteralPath $envPath) } else { @() }
  $lines = @($lines | Where-Object { $_ -notmatch '^\s*ELON_DESKTOP_REVIEW_(CREDENTIAL|PUBLIC_KEYS)=' })
  $lines += 'ELON_DESKTOP_REVIEW_PUBLIC_KEYS=' + ($Values -join ';')
  $tmp = $envPath + '.desktop-review.tmp'
  $parent = Split-Path -Parent $envPath
  if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
  [IO.File]::WriteAllLines($tmp, $lines, (New-Object Text.UTF8Encoding($false)))
  Move-Item -LiteralPath $tmp -Destination $envPath -Force
}

try {
  if ($Action -in @('Prepare','Validate','Commit')) { Assert-SeparateIdentities }
  if ($Action -eq 'Prepare') {
    New-Item -ItemType Directory -Path $stateRoot -Force | Out-Null
    $existing = Read-State $activePath
    if ($existing -and -not $Rotate) { Write-Result 'already_active' 'idempotent; use -Rotate to stage a replacement'; exit 0 }
    $cert = New-SelfSignedCertificate -Subject 'CN=Elon Desktop Review Signing' -CertStoreLocation 'Cert:\CurrentUser\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -KeyExportPolicy NonExportable -KeyUsage DigitalSignature -NotAfter (Get-Date).AddYears(2)
    Public-Record $cert | ConvertTo-Json | Set-Content -LiteralPath $stagePath -Encoding UTF8
    Write-Result 'prepared' 'non-exportable signing key staged; no runtime state changed'
    exit 0
  }
  if ($Action -eq 'Validate') {
    $stage = Read-State $stagePath
    if (-not $stage) { throw 'no prepared credential' }
    $cert = Get-Item ('Cert:\CurrentUser\My\' + $stage.thumbprint)
    if (-not $cert.HasPrivateKey -or -not (Test-Path -LiteralPath $stage.key_file)) { throw 'prepared private key is unavailable' }
    Write-Result 'valid' 'staged key is present; identity separation is valid; runtime unchanged'
    exit 0
  }
  if ($Action -eq 'Commit') {
    $stage = Read-State $stagePath
    if (-not $stage) { throw 'no validated prepared credential' }
    $cert = Get-Item ('Cert:\CurrentUser\My\' + $stage.thumbprint)
    if (-not $cert.HasPrivateKey) { throw 'prepared private key is unavailable' }
    $old = Read-State $activePath
    if ($old) { $old | ConvertTo-Json | Set-Content -LiteralPath $backupPath -Encoding UTF8 }
    & icacls.exe $stage.key_file /inheritance:r /grant:r ('*' + $DesktopIdentitySid + ':(R)') /deny ('*' + $ExecutorIdentitySid + ':(R,W,D)') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'private-key ACL commit failed; old public state retained' }
    $values = @($stage.public_value)
    if ($old) { $values += $old.public_value }
    Set-PublicValues $values
    $stage | ConvertTo-Json | Set-Content -LiteralPath $activePath -Encoding UTF8
    Remove-Item -LiteralPath $stagePath -Force
    Write-Result 'committed' 'public verifier committed atomically; previous key retained for rotation window'
    exit 0
  }
  if ($Action -eq 'Rollback') {
    $backup = Read-State $backupPath
    if (-not $backup) { throw 'no rollback state is available' }
    Set-PublicValues @($backup.public_value)
    $backup | ConvertTo-Json | Set-Content -LiteralPath $activePath -Encoding UTF8
    Write-Result 'rolled_back' 'previous public verifier restored; restart is still operator-controlled'
    exit 0
  }
  $active = Read-State $activePath
  $prepared = Read-State $stagePath
  $legacy = [Environment]::GetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL','User') -or [Environment]::GetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL','Machine')
  $detail = 'active={0}; prepared={1}; legacy_global_secret={2}; restart_required={3}' -f [bool]$active,[bool]$prepared,[bool]$legacy,[bool]$active
  Write-Result 'diagnosed' $detail
} catch {
  Write-Result 'failed' ([string]$_.Exception.Message)
  exit 1
}

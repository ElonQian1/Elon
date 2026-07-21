[CmdletBinding(SupportsShouldProcess = $true)]
param(
  [ValidateSet('Prepare','Validate','Commit','Diagnose','Rollback')][string]$Action = 'Diagnose',
  [string]$DesktopIdentitySid,
  [string]$ExecutorIdentitySid,
  [Parameter(Mandatory=$true)][string]$StateRoot,
  [Parameter(Mandatory=$true)][string]$InstallRoot,
  [string]$CertificateStoreLocation = 'Cert:\CurrentUser\My',
  [switch]$Rotate
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$StateRoot = [IO.Path]::GetFullPath($StateRoot)
$InstallRoot = [IO.Path]::GetFullPath($InstallRoot)
$stagePath = Join-Path $StateRoot 'prepared.json'
$activePath = Join-Path $StateRoot 'active.json'
$backupPath = Join-Path $StateRoot 'rollback.json'
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
  $desktopSid = New-Object Security.Principal.SecurityIdentifier($DesktopIdentitySid)
  $null = New-Object Security.Principal.SecurityIdentifier($ExecutorIdentitySid)
  $currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User
  if (-not $currentSid.Equals($desktopSid)) {
    throw 'fail_closed: credential operation must run as DesktopIdentitySid'
  }
}
function Get-KeyFile([Security.Cryptography.X509Certificates.X509Certificate2]$Cert) {
  $rsa = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($Cert)
  if ($null -eq $rsa -or $null -eq $rsa.Key) { throw 'CNG private key is unavailable' }
  Join-Path $env:APPDATA ('Microsoft\Crypto\Keys\' + $rsa.Key.UniqueName)
}
function Set-PrivateAcl([string]$Path) {
  $desktop = New-Object Security.Principal.SecurityIdentifier($DesktopIdentitySid)
  $executor = New-Object Security.Principal.SecurityIdentifier($ExecutorIdentitySid)
  $acl = Get-Acl -LiteralPath $Path
  $acl.SetOwner($desktop)
  $acl.SetAccessRuleProtection($true, $false)
  foreach ($rule in @($acl.Access)) { [void]$acl.RemoveAccessRuleSpecific($rule) }
  $desktopRights = if ((Get-Item -LiteralPath $Path).PSIsContainer) { 'FullControl' } else { 'ReadAndExecute' }
  $acl.AddAccessRule((New-Object -TypeName Security.AccessControl.FileSystemAccessRule -ArgumentList $desktop,$desktopRights,'Allow'))
  $acl.AddAccessRule((New-Object -TypeName Security.AccessControl.FileSystemAccessRule -ArgumentList $executor,'FullControl','Deny'))
  Set-Acl -LiteralPath $Path -AclObject $acl
}
function Assert-PrivateAcl([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { throw "protected path is unavailable: $Path" }
  $acl = Get-Acl -LiteralPath $Path
  if ($acl.Owner -ne $DesktopIdentitySid) {
    $ownerSid = (New-Object Security.Principal.NTAccount($acl.Owner)).Translate([Security.Principal.SecurityIdentifier]).Value
    if ($ownerSid -ne $DesktopIdentitySid) { throw "ACL owner is not DesktopIdentitySid: $Path" }
  }
  if (-not $acl.AreAccessRulesProtected) { throw "ACL inheritance is still enabled: $Path" }
  $allowsDesktop = @($acl.Access | Where-Object {
    $_.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -eq $DesktopIdentitySid -and $_.AccessControlType -eq 'Allow' -and
    ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::Read) })
  $deniesExecutor = @($acl.Access | Where-Object {
    $_.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -eq $ExecutorIdentitySid -and $_.AccessControlType -eq 'Deny' -and
    ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::Read) })
  if (-not $allowsDesktop -or -not $deniesExecutor) { throw "required Desktop allow / executor deny ACL is missing: $Path" }
}
function Public-Record([Security.Cryptography.X509Certificates.X509Certificate2]$Cert) {
  $rsa = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPublicKey($Cert)
  $p = $rsa.ExportParameters($false)
  $fingerprint = [Security.Cryptography.SHA256]::Create().ComputeHash($p.Modulus)
  $keyId = -join ($fingerprint[0..7] | ForEach-Object { $_.ToString('x2') })
  [pscustomobject]@{
    schema=3; key_id=$keyId; thumbprint=$Cert.Thumbprint
    public_value=($keyId + ':' + [Convert]::ToBase64String($p.Modulus) + ':' + [Convert]::ToBase64String($p.Exponent))
    key_file=(Get-KeyFile $Cert); desktop_sid=$DesktopIdentitySid; executor_sid=$ExecutorIdentitySid
    state_root=$StateRoot; install_root=$InstallRoot; certificate_store=$CertificateStoreLocation
  }
}
function Read-State([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return $null }
  Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}
function Write-JsonAtomic([string]$Path, [object]$Value) {
  $tmp = $Path + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
  [IO.File]::WriteAllText($tmp, ($Value | ConvertTo-Json), (New-Object Text.UTF8Encoding($false)))
  Move-Item -LiteralPath $tmp -Destination $Path -Force
}
function Set-PublicValues([string[]]$Values) {
  $lines = if (Test-Path -LiteralPath $envPath) { @(Get-Content -LiteralPath $envPath) } else { @() }
  $lines = @($lines | Where-Object { $_ -notmatch '^\s*ELON_DESKTOP_REVIEW_(CREDENTIAL|PUBLIC_KEYS|NONCE_LEDGER|ALLOW_V2)=' })
  $lines += 'ELON_DESKTOP_REVIEW_PUBLIC_KEYS=' + ($Values -join ';')
  $lines += 'ELON_DESKTOP_REVIEW_NONCE_LEDGER=' + (Join-Path $InstallRoot '_internal\desktop-review-nonces.json')
  $lines += 'ELON_DESKTOP_REVIEW_ALLOW_V2=0'
  $tmp = $envPath + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
  $parent = Split-Path -Parent $envPath
  if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
  [IO.File]::WriteAllLines($tmp, $lines, (New-Object Text.UTF8Encoding($false)))
  Move-Item -LiteralPath $tmp -Destination $envPath -Force
}
function Assert-State([object]$State) {
  if (-not $State -or $State.schema -notin @(2,3)) { throw 'credential state schema is unsupported' }
  if ([IO.Path]::GetFullPath([string]$State.state_root) -ne $StateRoot -or
      [IO.Path]::GetFullPath([string]$State.install_root) -ne $InstallRoot -or
      [string]$State.certificate_store -ne $CertificateStoreLocation) {
    throw 'fail_closed: StateRoot, InstallRoot, or certificate store does not match prepared state'
  }
  if ($State.desktop_sid -ne $DesktopIdentitySid -or $State.executor_sid -ne $ExecutorIdentitySid) {
    throw 'fail_closed: prepared credential identity does not match requested identity'
  }
  $cert = Get-Item ($CertificateStoreLocation + '\' + $State.thumbprint)
  if (-not $cert.HasPrivateKey) { throw 'prepared private key is unavailable' }
  $keyFile = Get-KeyFile $cert
  if ([IO.Path]::GetFullPath($keyFile) -ne [IO.Path]::GetFullPath([string]$State.key_file)) {
    throw 'prepared private key path does not match certificate'
  }
  Assert-PrivateAcl $StateRoot
  Assert-PrivateAcl $keyFile
  return $cert
}

try {
  if ($Action -in @('Prepare','Validate','Commit','Rollback')) { Assert-SeparateIdentities }
  if ($Action -eq 'Prepare') {
    if (-not (Test-Path -LiteralPath $StateRoot)) { New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null }
    Set-PrivateAcl $StateRoot
    $existing = Read-State $activePath
    if ($existing -and -not $Rotate) { Write-Result 'already_active' 'idempotent; use -Rotate to stage a replacement'; exit 0 }
    $cert = New-SelfSignedCertificate -Subject 'CN=Elon Desktop Review Signing' -CertStoreLocation $CertificateStoreLocation -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -KeyExportPolicy NonExportable -KeyUsage DigitalSignature -NotAfter (Get-Date).AddYears(2)
    $record = Public-Record $cert
    Set-PrivateAcl $record.key_file
    Write-JsonAtomic $stagePath $record
    Write-Result 'prepared' 'non-exportable signing key staged with protected ACL; no runtime state changed'
    exit 0
  }
  if ($Action -eq 'Validate') {
    $stage = Read-State $stagePath
    $null = Assert-State $stage
    Write-Result 'valid' 'certificate ownership, private-key ACL, identity separation, and roots are valid; runtime unchanged'
    exit 0
  }
  if ($Action -eq 'Commit') {
    $stage = Read-State $stagePath
    $null = Assert-State $stage
    $old = Read-State $activePath
    if ($old) { Write-JsonAtomic $backupPath $old }
    $values = @($stage.public_value)
    if ($old) { $values += $old.public_value }
    $oldEnv = if (Test-Path -LiteralPath $envPath) { [IO.File]::ReadAllBytes($envPath) } else { $null }
    try {
      Set-PublicValues $values
      Write-JsonAtomic $activePath $stage
    } catch {
      if ($null -ne $oldEnv) { [IO.File]::WriteAllBytes($envPath, $oldEnv) }
      elseif (Test-Path -LiteralPath $envPath) { Remove-Item -LiteralPath $envPath -Force }
      if ($old) { Write-JsonAtomic $activePath $old }
      throw
    }
    Remove-Item -LiteralPath $stagePath -Force
    Write-Result 'committed' 'public verifier committed atomically; previous key retained for rotation window'
    exit 0
  }
  if ($Action -eq 'Rollback') {
    $backup = Read-State $backupPath
    if (-not $backup) { throw 'no rollback state is available' }
    $null = Assert-State $backup
    $current = Read-State $activePath
    $oldEnv = if (Test-Path -LiteralPath $envPath) { [IO.File]::ReadAllBytes($envPath) } else { $null }
    try {
      Set-PublicValues @($backup.public_value)
      Write-JsonAtomic $activePath $backup
    } catch {
      if ($null -ne $oldEnv) { [IO.File]::WriteAllBytes($envPath, $oldEnv) }
      if ($current) { Write-JsonAtomic $activePath $current }
      throw
    }
    Write-Result 'rolled_back' 'previous public verifier restored; restart is still operator-controlled'
    exit 0
  }
  $active = Read-State $activePath
  $prepared = Read-State $stagePath
  $legacy = [Environment]::GetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL','User') -or [Environment]::GetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL','Machine')
  $detail = 'active={0}; prepared={1}; legacy_global_secret={2}; restart_required={3}; explicit_roots=true' -f [bool]$active,[bool]$prepared,[bool]$legacy,[bool]$active
  Write-Result 'diagnosed' $detail
} catch {
  Write-Result 'failed' ([string]$_.Exception.Message)
  exit 1
}

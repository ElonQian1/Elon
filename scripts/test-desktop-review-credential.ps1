$ErrorActionPreference = 'Stop'
$credentialScript = Join-Path $PSScriptRoot 'desktop-review-credential.ps1'
$ticketScript = Join-Path $PSScriptRoot 'new-desktop-review-ticket.ps1'
foreach ($script in @($credentialScript, $ticketScript)) {
  $tokens = $null; $errors = $null
  [void][Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$errors)
  if ($errors.Count) { throw "PowerShell parse failed: $script : $($errors[0].Message)" }
}

$desktopSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$executorSid = 'S-1-5-20' # NETWORK SERVICE; deliberately distinct from the interactive Desktop identity.
$smokeRoot = Join-Path ([IO.Path]::GetTempPath()) ('elon-desktop-review-smoke-' + [Guid]::NewGuid().ToString('N'))
$stateRoot = Join-Path $smokeRoot 'desktop-state'
$installRoot = Join-Path $smokeRoot 'node-install'
$createdThumbprints = New-Object Collections.Generic.List[string]

function Invoke-Credential([string]$Action, [switch]$Rotate) {
  $args = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$credentialScript,'-Action',$Action,
    '-DesktopIdentitySid',$desktopSid,'-ExecutorIdentitySid',$executorSid,
    '-StateRoot',$stateRoot,'-InstallRoot',$installRoot)
  if ($Rotate) { $args += '-Rotate' }
  $output = & powershell.exe @args 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { throw "$Action failed: $output" }
  return ($output | ConvertFrom-Json)
}

try {
  New-Item -ItemType Directory -Path $smokeRoot -Force | Out-Null
  $sameSid = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $credentialScript -Action Validate `
    -DesktopIdentitySid $desktopSid -ExecutorIdentitySid $desktopSid -StateRoot $stateRoot -InstallRoot $installRoot 2>&1 | Out-String
  if ($LASTEXITCODE -eq 0 -or $sameSid -notmatch 'fail_closed') { throw 'same-identity validation did not fail closed' }

  $prepare = Invoke-Credential Prepare
  if ($prepare.status -ne 'prepared') { throw 'prepare did not stage a credential' }
  $stage = Get-Content -Raw -LiteralPath (Join-Path $stateRoot 'prepared.json') | ConvertFrom-Json
  $createdThumbprints.Add([string]$stage.thumbprint)
  $validate = Invoke-Credential Validate
  if ($validate.status -ne 'valid') { throw 'real ACL validation did not pass' }
  $commit = Invoke-Credential Commit
  if ($commit.status -ne 'committed') { throw 'commit did not activate the verifier' }

  [byte[]]$reviewBytes = [Text.UTF8Encoding]::new($false).GetBytes('{"verdict":"accepted","summary":"钱一龙"}')
  $sha = [Security.Cryptography.SHA256]::Create()
  try { $bodyHash = -join ($sha.ComputeHash($reviewBytes) | ForEach-Object { $_.ToString('x2') }) } finally { $sha.Dispose() }
  $ticket = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ticketScript `
    -OwnerUserId 'owner-smoke' -TaskId 'task-smoke' -Method POST `
    -EndpointPath '/api/local-tasks/task-smoke/supervision/desktop-review' -BodySha256 $bodyHash `
    -StateRoot $stateRoot -InstallRoot $installRoot
  if ($LASTEXITCODE -ne 0 -or $ticket -notmatch '^v3\.') { throw 'Desktop v3 signing smoke failed' }
  $ErrorActionPreference = 'Continue'
  $wrongRoot = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ticketScript `
    -OwnerUserId 'owner-smoke' -TaskId 'task-smoke' -Method POST `
    -EndpointPath '/api/local-tasks/task-smoke/supervision/desktop-review' -BodySha256 $bodyHash `
    -StateRoot $stateRoot -InstallRoot (Join-Path $smokeRoot 'wrong-install') 2>&1 | Out-String
  $wrongRootExit = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  if ($wrongRootExit -eq 0 -or $wrongRoot -notmatch 'fail_closed') { throw 'explicit root mismatch did not fail closed' }

  $rotate = Invoke-Credential Prepare -Rotate
  if ($rotate.status -ne 'prepared') { throw 'rotation prepare failed' }
  $rotated = Get-Content -Raw -LiteralPath (Join-Path $stateRoot 'prepared.json') | ConvertFrom-Json
  $createdThumbprints.Add([string]$rotated.thumbprint)
  $null = Invoke-Credential Validate
  $null = Invoke-Credential Commit
  $publicLine = Get-Content -LiteralPath (Join-Path $installRoot '_internal\node-agent.env') |
    Where-Object { $_ -match '^ELON_DESKTOP_REVIEW_PUBLIC_KEYS=' }
  if (($publicLine -split ';').Count -ne 2) { throw 'rotation window did not retain two public keys' }
  $nodeEnvLines = @(Get-Content -LiteralPath (Join-Path $installRoot '_internal\node-agent.env'))
  if (-not @($nodeEnvLines | Where-Object { $_ -like 'ELON_DESKTOP_REVIEW_NONCE_LEDGER=*' }) -or
      -not @($nodeEnvLines | Where-Object { $_ -eq 'ELON_DESKTOP_REVIEW_ALLOW_V2=0' })) { throw 'v3 verifier defaults are missing' }
  $rollback = Invoke-Credential Rollback
  if ($rollback.status -ne 'rolled_back') { throw 'rollback failed' }
  $rolledBackLine = Get-Content -LiteralPath (Join-Path $installRoot '_internal\node-agent.env') |
    Where-Object { $_ -match '^ELON_DESKTOP_REVIEW_PUBLIC_KEYS=' }
  if (($rolledBackLine -split ';').Count -ne 1) { throw 'rollback did not restore one previous public key' }

  $sources = (Get-Content -Raw -LiteralPath $credentialScript) + (Get-Content -Raw -LiteralPath $ticketScript)
  if ($sources -match 'SetEnvironmentVariable') { throw 'global environment persistence detected' }
  if ($sources -match 'Write-(Host|Output)[^\r\n]*(private|signature|ticket)') { throw 'possible secret output detected' }
  if ((Get-Content -Raw -LiteralPath (Join-Path $installRoot '_internal\node-agent.env')) -match 'PRIVATE|BEGIN|thumbprint|key_file') {
    throw 'private credential metadata leaked into NodeAgent state'
  }
  Write-Output 'DESKTOP_REVIEW_CREDENTIAL_TEST=passed'
} finally {
  foreach ($thumbprint in $createdThumbprints) {
    Remove-Item -LiteralPath ('Cert:\CurrentUser\My\' + $thumbprint) -Force -ErrorAction SilentlyContinue
  }
  Remove-Item -LiteralPath $smokeRoot -Recurse -Force -ErrorAction SilentlyContinue
}

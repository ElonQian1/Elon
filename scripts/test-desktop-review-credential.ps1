$ErrorActionPreference = 'Stop'
$scripts = @(
  (Join-Path $PSScriptRoot 'desktop-review-credential.ps1'),
  (Join-Path $PSScriptRoot 'new-desktop-review-ticket.ps1')
)
foreach ($script in $scripts) {
  $tokens = $null; $errors = $null
  [void][Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$errors)
  if ($errors.Count) { throw "PowerShell parse failed: $script : $($errors[0].Message)" }
}
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$output = & $scripts[0] -Action Validate -DesktopIdentitySid $sid -ExecutorIdentitySid $sid 2>&1 | Out-String
if ($LASTEXITCODE -eq 0 -or $output -notmatch 'fail_closed') { throw 'same-identity validation did not fail closed' }
$sources = ($scripts | ForEach-Object { Get-Content -Raw -LiteralPath $_ }) -join "`n"
if ($sources -match 'SetEnvironmentVariable') { throw 'global environment persistence detected' }
if ($sources -match 'Write-(Host|Output)[^\r\n]*(private|signature|ticket)') { throw 'possible secret output detected' }
Write-Output 'DESKTOP_REVIEW_CREDENTIAL_TEST=passed'

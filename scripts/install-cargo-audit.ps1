param(
    [string]$Version = "0.22.2"
)

$ErrorActionPreference = "Stop"

function Stop-CargoAuditInstall {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Invoke-CargoAuditVersion {
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & cargo audit --version 2>$null
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }

    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = if ($null -eq $output) { "" } else { ($output -join "`n").Trim() }
    }
}

if (-not (Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue)) {
    Stop-CargoAuditInstall "cargo is required before installing cargo-audit."
}

$current = Invoke-CargoAuditVersion
$targetPattern = "(^|\s)$([regex]::Escape($Version))(\s|$)"
if ($current.ExitCode -eq 0 -and $current.Output -match $targetPattern) {
    Write-Host "CARGO_AUDIT_INSTALL=present version=$Version"
    exit 0
}

if ($current.ExitCode -eq 0) {
    Write-Host "CARGO_AUDIT_INSTALL=reinstall current='$($current.Output)' target=$Version"
} else {
    Write-Host "CARGO_AUDIT_INSTALL=install target=$Version"
}

& cargo install cargo-audit --version $Version --locked --force
if ($LASTEXITCODE -ne 0) {
    Stop-CargoAuditInstall "cargo install cargo-audit failed."
}

$installed = Invoke-CargoAuditVersion
if ($installed.ExitCode -ne 0 -or $installed.Output -notmatch $targetPattern) {
    Stop-CargoAuditInstall "cargo-audit installation did not produce the expected version $Version. Output: $($installed.Output)"
}

Write-Host "CARGO_AUDIT_INSTALL=installed version=$Version"

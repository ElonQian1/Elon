param(
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Write-Result {
    param(
        [bool]$Ok,
        [string]$Message,
        [string]$Path = "",
        [string]$Version = ""
    )

    if ($Json) {
        [ordered]@{
            ok = $Ok
            message = $Message
            path = $Path
            version = $Version
            install_command = "winget install --id Microsoft.PowerShell --source winget"
            verify_command = "pwsh -NoProfile -Command '`$PSVersionTable.PSVersion'"
        } | ConvertTo-Json -Depth 4
    } else {
        if ($Ok) {
            Write-Host "OK: PowerShell 7 is available." -ForegroundColor Green
            Write-Host "pwsh: $Path"
            Write-Host "version: $Version"
        } else {
            Write-Host "PowerShell 7 is required for scripts with '#requires -Version 7.0'." -ForegroundColor Yellow
            Write-Host $Message
            Write-Host ""
            Write-Host "Install:"
            Write-Host "  winget install --id Microsoft.PowerShell --source winget"
            Write-Host ""
            Write-Host "Verify:"
            Write-Host "  pwsh -NoProfile -Command '`$PSVersionTable.PSVersion'"
            Write-Host ""
            Write-Host "Do not remove '#requires -Version 7.0' or downgrade PS7 scripts for Windows PowerShell 5.1."
        }
    }
}

$pwshCommand = Get-Command pwsh -ErrorAction SilentlyContinue
if (-not $pwshCommand) {
    Write-Result -Ok $false -Message "pwsh was not found on PATH."
    exit 1
}

$versionText = (& $pwshCommand.Source -NoProfile -NoLogo -Command '$PSVersionTable.PSVersion.ToString()').Trim()
try {
    $version = [version]$versionText
} catch {
    Write-Result -Ok $false -Message "pwsh was found but its version could not be parsed: $versionText" -Path $pwshCommand.Source -Version $versionText
    exit 1
}

if ($version -lt [version]"7.0") {
    Write-Result -Ok $false -Message "pwsh version is lower than 7.0." -Path $pwshCommand.Source -Version $versionText
    exit 1
}

Write-Result -Ok $true -Message "PowerShell 7 is available." -Path $pwshCommand.Source -Version $versionText
exit 0

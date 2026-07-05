param(
    [switch]$KeepTemp
)

$ErrorActionPreference = "Stop"

function Save-Env {
    param([string[]]$Names)
    $state = @{}
    foreach ($name in $Names) {
        $state[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    return $state
}

function Restore-Env {
    param(
        [hashtable]$State,
        [string[]]$Names
    )
    foreach ($name in $Names) {
        [Environment]::SetEnvironmentVariable($name, $State[$name], "Process")
    }
}

function Get-DataRoot {
    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
            return $env:LOCALAPPDATA
        }
        return [System.IO.Path]::GetTempPath().TrimEnd("\", "/")
    }
    if (-not [string]::IsNullOrWhiteSpace($env:XDG_DATA_HOME)) {
        return $env:XDG_DATA_HOME
    }
    if (-not [string]::IsNullOrWhiteSpace($env:HOME)) {
        return (Join-Path $env:HOME ".local/share")
    }
    return [System.IO.Path]::GetTempPath().TrimEnd("\", "/")
}

function Test-ManagedVaultPath {
    param([string]$Path)
    $full = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetFullPath((Join-Path (Get-DataRoot) "Elon/codex-vault"))
    return $full.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)
}

function Test-ManagedLeaseExpired {
    param([string]$CodexHome)
    $metaPath = Join-Path $CodexHome "elon-codex-vault-slot.json"
    if (-not (Test-Path -LiteralPath $metaPath)) {
        return $false
    }
    $meta = Get-Content -LiteralPath $metaPath -Raw | ConvertFrom-Json
    if ([string]::IsNullOrWhiteSpace($meta.lease_expires_at)) {
        return $false
    }
    return ([DateTimeOffset]::Parse($meta.lease_expires_at).UtcDateTime -le [DateTime]::UtcNow)
}

function Get-CodexChildHome {
    $value = $env:CODEX_HOME
    if (-not [string]::IsNullOrWhiteSpace($value) -and (Test-Path -LiteralPath $value)) {
        if ((Test-ManagedVaultPath $value) -and (Test-ManagedLeaseExpired $value)) {
            [Environment]::SetEnvironmentVariable("CODEX_HOME", $null, "Process")
        } else {
            return $value
        }
    }
    $profile = $env:USERPROFILE
    if ([string]::IsNullOrWhiteSpace($profile)) {
        $profile = $env:HOME
    }
    if (-not [string]::IsNullOrWhiteSpace($profile)) {
        $defaultHome = Join-Path $profile ".codex"
        if (Test-Path -LiteralPath $defaultHome) {
            return $defaultHome
        }
    }
    return $null
}

function Invoke-FakeCodex {
    param(
        [string]$FakeCliPath,
        [string]$SelectedHome,
        [string]$ExpectedHome,
        [string]$OutputPath
    )
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = "powershell.exe"
    $escaped = $FakeCliPath.Replace('"', '\"')
    $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$escaped`""
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.Environment["CODEX_HOME"] = $SelectedHome
    $psi.Environment["EXPECTED_CODEX_HOME"] = $ExpectedHome
    $psi.Environment["FAKE_CODEX_OUTPUT"] = $OutputPath
    $psi.Environment.Remove("OPENAI_API_KEY") | Out-Null
    $psi.Environment.Remove("CODEX_API_KEY") | Out-Null
    $psi.Environment.Remove("OPENAI_BASE_URL") | Out-Null
    $psi.Environment.Remove("OPENAI_API_BASE") | Out-Null
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    if ($proc.ExitCode -ne 0) {
        throw "fake codex failed exit=$($proc.ExitCode) stdout=$stdout stderr=$stderr"
    }
}

$envNames = @(
    "LOCALAPPDATA",
    "XDG_DATA_HOME",
    "USERPROFILE",
    "HOME",
    "CODEX_HOME",
    "OPENAI_API_KEY",
    "CODEX_API_KEY",
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE"
)
$savedEnv = Save-Env $envNames
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-codex-fake-cli-" + [Guid]::NewGuid().ToString("N"))

try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    $fakeCli = Join-Path $tempRoot "fake-codex.ps1"
    $fakeCliBody = @'
$payload = [ordered]@{
  ok = ($env:CODEX_HOME -eq $env:EXPECTED_CODEX_HOME)
  codex_home = $env:CODEX_HOME
  expected = $env:EXPECTED_CODEX_HOME
  openai_api_key_present = -not [string]::IsNullOrWhiteSpace($env:OPENAI_API_KEY)
  codex_api_key_present = -not [string]::IsNullOrWhiteSpace($env:CODEX_API_KEY)
}
$payload | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $env:FAKE_CODEX_OUTPUT -Encoding UTF8
if (-not $payload.ok -or $payload.openai_api_key_present -or $payload.codex_api_key_present) {
  exit 1
}
exit 0
'@
    Set-Content -LiteralPath $fakeCli -Value $fakeCliBody -Encoding UTF8

    $env:LOCALAPPDATA = $tempRoot
    $env:XDG_DATA_HOME = $tempRoot
    $env:USERPROFILE = (Join-Path $tempRoot "profile")
    $env:HOME = $env:USERPROFILE
    New-Item -ItemType Directory -Force -Path $env:USERPROFILE | Out-Null
    $defaultHome = Join-Path $env:USERPROFILE ".codex"
    New-Item -ItemType Directory -Force -Path $defaultHome | Out-Null

    $activeHome = Join-Path $tempRoot "Elon/codex-vault/slots/shared-provider/codex-home"
    New-Item -ItemType Directory -Force -Path $activeHome | Out-Null
    @{ slot_id = "shared-provider"; lease_expires_at = "2999-01-01T00:00:00+00:00" } |
        ConvertTo-Json -Compress |
        Set-Content -LiteralPath (Join-Path $activeHome "elon-codex-vault-slot.json") -Encoding UTF8
    $env:CODEX_HOME = $activeHome
    $activeSelected = Get-CodexChildHome
    if ($activeSelected -ne $activeHome) {
        throw "active managed CODEX_HOME was not selected"
    }
    $activeOut = Join-Path $tempRoot "active-result.json"
    Invoke-FakeCodex -FakeCliPath $fakeCli -SelectedHome $activeSelected -ExpectedHome $activeHome -OutputPath $activeOut

    $expiredHome = Join-Path $tempRoot "Elon/codex-vault/slots/expired-provider/codex-home"
    New-Item -ItemType Directory -Force -Path $expiredHome | Out-Null
    @{ slot_id = "expired-provider"; lease_expires_at = "2000-01-01T00:00:00+00:00" } |
        ConvertTo-Json -Compress |
        Set-Content -LiteralPath (Join-Path $expiredHome "elon-codex-vault-slot.json") -Encoding UTF8
    $env:CODEX_HOME = $expiredHome
    $expiredSelected = Get-CodexChildHome
    if ($expiredSelected -ne $defaultHome) {
        throw "expired managed CODEX_HOME did not fall back to default home"
    }
    $expiredOut = Join-Path $tempRoot "expired-result.json"
    Invoke-FakeCodex -FakeCliPath $fakeCli -SelectedHome $expiredSelected -ExpectedHome $defaultHome -OutputPath $expiredOut

    [ordered]@{
        ok = $true
        active_selected_under_managed_vault = (Test-ManagedVaultPath $activeSelected)
        active_child = Get-Content -LiteralPath $activeOut -Raw | ConvertFrom-Json
        expired_fell_back_to_default = ($expiredSelected -eq $defaultHome)
        expired_child = Get-Content -LiteralPath $expiredOut -Raw | ConvertFrom-Json
        temp_root = $(if ($KeepTemp) { $tempRoot } else { $null })
    } | ConvertTo-Json -Depth 6
} finally {
    Restore-Env -State $savedEnv -Names $envNames
    if (-not $KeepTemp -and (Test-Path -LiteralPath $tempRoot)) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

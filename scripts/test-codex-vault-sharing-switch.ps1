param(
    [string]$ProviderUserId,
    [string]$ProviderAccount,
    [string]$NodeUrl = "http://127.0.0.1:7799",
    [string]$Prompt = "Reply OK only. Do not call tools.",
    [int]$BadTimeoutSeconds = 90,
    [int]$SharedTimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ProviderUserId) -and [string]::IsNullOrWhiteSpace($ProviderAccount)) {
    throw "Pass -ProviderUserId or -ProviderAccount to select the sharing provider robot."
}

function Shorten-Text([string]$Value, [int]$Max) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return "" }
    $flat = ($Value.Trim() -replace "\s+", " ")
    if ($flat.Length -le $Max) { return $flat }
    return $flat.Substring([Math]::Max(0, $flat.Length - $Max), $Max)
}

function Invoke-NodeJson(
    [string]$Path,
    [string]$Method = "GET",
    [object]$Body = $null,
    [hashtable]$Headers = @{},
    [int]$TimeoutSec = 30
) {
    $uri = $NodeUrl.TrimEnd("/") + $Path
    $params = @{
        Uri = $uri
        Method = $Method
        Headers = $Headers
        TimeoutSec = $TimeoutSec
    }
    if ($null -ne $Body) {
        $params.ContentType = "application/json"
        $params.Body = ($Body | ConvertTo-Json -Depth 8 -Compress)
    }
    Invoke-RestMethod @params
}

function Invoke-NodeJsonWithFallback(
    [string[]]$Paths,
    [string]$Method,
    [object]$Body,
    [hashtable]$Headers,
    [int]$TimeoutSec
) {
    $lastError = $null
    foreach ($path in $Paths) {
        try {
            return Invoke-NodeJson -Path $path -Method $Method -Body $Body -Headers $Headers -TimeoutSec $TimeoutSec
        } catch {
            $lastError = $_
        }
    }
    throw $lastError
}

$status = Invoke-NodeJson -Path "/api/status" -Headers @{ "Sec-Fetch-Site" = "same-origin" } -TimeoutSec 15
$localAdminToken = $status.local_admin_token
if ([string]::IsNullOrWhiteSpace($localAdminToken)) {
    throw "Local node did not expose local admin token. Confirm this runs from the trusted local PC context."
}
$codexExe = $status.codex_cli.path
if ([string]::IsNullOrWhiteSpace($codexExe) -or -not (Test-Path -LiteralPath $codexExe)) {
    throw "No runnable Codex CLI was found."
}

$tempRootPath = [System.IO.Path]::GetTempPath().TrimEnd("\")
Get-ChildItem -LiteralPath $tempRootPath -Directory -Filter "elon-codex-sharing-proof-*" -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName.StartsWith($tempRootPath, [System.StringComparison]::OrdinalIgnoreCase) } |
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

$root = Join-Path $env:TEMP ("elon-codex-sharing-proof-" + [guid]::NewGuid().ToString("N"))
$badCodexHome = Join-Path $root "bad-home"
$workDir = Join-Path $root "work"
New-Item -ItemType Directory -Force -Path $badCodexHome, $workDir | Out-Null

$badAuth = '{"tokens":{"id_token":"invalid","access_token":"invalid","refresh_token":"invalid","account_id":"invalid"},"last_refresh":"2000-01-01T00:00:00Z"}'
Set-Content -LiteralPath (Join-Path $badCodexHome "auth.json") -Value $badAuth -Encoding UTF8

function Invoke-CodexProbe([string]$CodexHomePath, [string]$Label, [int]$TimeoutSeconds) {
    $outFile = Join-Path $root ("$Label-last.txt")
    $timeoutFlag = Join-Path $root ("$Label-timeout.flag")
    $codexArgs = @(
        "exec",
        "--skip-git-repo-check",
        "--ignore-user-config",
        "--ignore-rules",
        "--ephemeral",
        "--json",
        "--sandbox", "read-only",
        "-C", $workDir,
        "-o", $outFile,
        $Prompt
    )
    $allFile = Join-Path $root ("$Label-output.txt")
    $savedProbeEnv = @{}
    foreach ($name in @("CODEX_HOME", "OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL", "OPENAI_API_BASE")) {
        $savedProbeEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    [Environment]::SetEnvironmentVariable("CODEX_HOME", $CodexHomePath, "Process")
    foreach ($name in @("OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL", "OPENAI_API_BASE")) {
        [Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    $watchdog = $null
    if ($TimeoutSeconds -gt 0) {
        $watchdog = Start-Job -ScriptBlock {
            param(
                [int]$ProbeTimeoutSeconds,
                [string]$ProbeRoot,
                [string]$ProbeTimeoutFlag
            )
            Start-Sleep -Seconds $ProbeTimeoutSeconds
            $matches = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
                Where-Object {
                    -not [string]::IsNullOrWhiteSpace($_.CommandLine) -and
                    $_.CommandLine.IndexOf($ProbeRoot, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
                })
            if ($matches.Count -gt 0) {
                "timed_out" | Set-Content -LiteralPath $ProbeTimeoutFlag -Encoding UTF8
                foreach ($process in $matches) {
                    try {
                        Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
                    } catch {}
                }
            }
        } -ArgumentList $TimeoutSeconds, $root, $timeoutFlag
    }
    try {
        Push-Location $workDir
        try {
            $oldErrorActionPreference = $ErrorActionPreference
            $oldNativeCommandUseErrorActionPreference = $null
            $hasNativePreference = Test-Path variable:PSNativeCommandUseErrorActionPreference
            if ($hasNativePreference) {
                $oldNativeCommandUseErrorActionPreference = $PSNativeCommandUseErrorActionPreference
                $PSNativeCommandUseErrorActionPreference = $false
            }
            $ErrorActionPreference = "Continue"
            & $codexExe @codexArgs *> $allFile
            $exitCode = $LASTEXITCODE
            $ErrorActionPreference = $oldErrorActionPreference
            if ($hasNativePreference) {
                $PSNativeCommandUseErrorActionPreference = $oldNativeCommandUseErrorActionPreference
            }
        } finally {
            if ($null -ne $watchdog) {
                if ($watchdog.State -eq "Running") {
                    Stop-Job -Job $watchdog -ErrorAction SilentlyContinue
                }
                Receive-Job -Job $watchdog -ErrorAction SilentlyContinue | Out-Null
                Remove-Job -Job $watchdog -Force -ErrorAction SilentlyContinue
            }
            if ($null -ne $oldErrorActionPreference) {
                $ErrorActionPreference = $oldErrorActionPreference
            }
            if ($hasNativePreference) {
                $PSNativeCommandUseErrorActionPreference = $oldNativeCommandUseErrorActionPreference
            }
            Pop-Location
        }
    } finally {
        foreach ($name in $savedProbeEnv.Keys) {
            [Environment]::SetEnvironmentVariable($name, $savedProbeEnv[$name], "Process")
        }
    }
    $output = if (Test-Path -LiteralPath $allFile) { Get-Content -LiteralPath $allFile -Raw } else { "" }
    $last = if (Test-Path -LiteralPath $outFile) { Get-Content -LiteralPath $outFile -Raw } else { "" }
    $timedOut = Test-Path -LiteralPath $timeoutFlag
    if ($timedOut -and $null -eq $exitCode) {
        $exitCode = 124
    }
    [pscustomobject]@{
        label = $Label
        exit_code = $exitCode
        timed_out = $timedOut
        timeout_enforced = ($TimeoutSeconds -gt 0)
        timeout_seconds = $TimeoutSeconds
        success = (($exitCode -eq 0) -and -not $timedOut)
        last_message = Shorten-Text $last 200
        stdout_tail = Shorten-Text $output 700
        stderr_tail = $(if ($timedOut) { "Codex probe timed out after $TimeoutSeconds seconds." } else { "" })
    }
}

$restore = $null
$clear = $null
try {
    $badProbe = Invoke-CodexProbe -CodexHomePath $badCodexHome -Label "bad" -TimeoutSeconds $BadTimeoutSeconds
    $headers = @{ "x-elon-local-admin-token" = $localAdminToken }
    $restoreBody = @{
        provider_user_id = if ([string]::IsNullOrWhiteSpace($ProviderUserId)) { $null } else { $ProviderUserId }
        provider_account = if ([string]::IsNullOrWhiteSpace($ProviderAccount)) { $null } else { $ProviderAccount }
        purpose = "robot_codex_vault_switch_probe"
    }
    $restore = Invoke-NodeJsonWithFallback `
        -Paths @("/api/codex-vault/sharing/restore", "/api/codex-vault/emergency-restore") `
        -Method "POST" `
        -Body $restoreBody `
        -Headers $headers `
        -TimeoutSec 60
    $sharedCodexHome = $restore.local.active_codex_home
    if ([string]::IsNullOrWhiteSpace($sharedCodexHome)) {
        throw "Sharing restore did not set an active CODEX_HOME."
    }
    $sharedProbe = Invoke-CodexProbe -CodexHomePath $sharedCodexHome -Label "shared" -TimeoutSeconds $SharedTimeoutSeconds
    $clear = Invoke-NodeJson -Path "/api/codex-vault/clear" -Method "POST" -Headers $headers -TimeoutSec 60
    $finalStatus = Invoke-NodeJson -Path "/api/status" -Headers @{ "Sec-Fetch-Site" = "same-origin" } -TimeoutSec 15
    $passed = (-not $badProbe.success) -and $sharedProbe.success -and ($null -eq $finalStatus.codex_vault.active_codex_home)
    $result = [pscustomobject]@{
        ok = $passed
        bad_probe = $badProbe
        restore_ok = $restore.ok
        lease_id = $restore.lease_id
        shared_account_hint_hash = $restore.account_hint_hash
        shared_home_under_managed_vault = $sharedCodexHome.StartsWith((Join-Path $env:LOCALAPPDATA "Elon\codex-vault"), [System.StringComparison]::OrdinalIgnoreCase)
        shared_probe = $sharedProbe
        clear_attempted = $clear.cloud_clear.attempted
        clear_ok = $clear.cloud_clear.ok
        final_active_home = $finalStatus.codex_vault.active_codex_home
        final_managed_slots_count = @($finalStatus.codex_vault.managed_slots).Count
        final_default_auth_present = $finalStatus.codex_vault.default_auth.present
    }
    $result | ConvertTo-Json -Depth 8
    if (-not $passed) { exit 1 }
} finally {
    if ($null -ne $restore -and $null -eq $clear) {
        try {
            Invoke-NodeJson -Path "/api/codex-vault/clear" -Method "POST" -Headers @{ "x-elon-local-admin-token" = $localAdminToken } -TimeoutSec 60 | Out-Null
        } catch {}
    }
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

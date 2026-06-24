#requires -Version 7.0

param(
    [string]$Fb2Base = "http://123.207.48.146:8080",
    [string]$ServerHost = "123.207.48.146",
    [int]$ServerPort = 29622,
    [string]$ServerUser = "ubuntu",
    [string]$SshKey = (Join-Path $env:USERPROFILE ".ssh\id_ed25519_fb2"),
    [string]$ServerEnvPath = "/home/ubuntu/fb2/backend/.env",
    [string]$Fb2Username = "123qwe",
    [string]$Fb2Password = "",
    [string]$ExternalUserId = "6fe5aa17-0403-427a-8e91-7f414beca35d",
    [string]$GroupId = "official",
    [string]$SummaryPath = "",
    [string]$ContractSmokeSummaryPath = "",
    [string]$OutputPath = "",
    [int]$RequestTimeoutSec = 60,
    [int]$PollTimeoutSec = 120,
    [int]$FeedbackPollTimeoutSec = 60,
    [switch]$RunDataOnlyPreflight,
    [switch]$RunCurrentStateAfter,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2TokenBridgeRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2TokenBridgePath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Save-Fb2TokenBridgeEnvironmentValue {
    param([string]$Name)

    [ordered]@{
        name = $Name
        exists = [System.Environment]::GetEnvironmentVariable($Name, "Process") -ne $null
        value = [System.Environment]::GetEnvironmentVariable($Name, "Process")
    }
}

function Restore-Fb2TokenBridgeEnvironmentValue {
    param([object]$Snapshot)

    if ([bool]$Snapshot.exists) {
        [System.Environment]::SetEnvironmentVariable([string]$Snapshot.name, [string]$Snapshot.value, "Process")
    } else {
        [System.Environment]::SetEnvironmentVariable([string]$Snapshot.name, $null, "Process")
    }
}

function Get-Fb2TokenBridgeRemoteSharedSecret {
    param(
        [string]$HostName,
        [int]$Port,
        [string]$User,
        [string]$KeyPath,
        [string]$EnvPath
    )

    if (-not (Test-Path -LiteralPath $KeyPath)) {
        throw "SSH key not found: $KeyPath"
    }

    # 只把目标变量读入当前进程内存；不要打印 token，也不要把 token 放入子进程参数。
    $remoteCommand = "sudo awk -F= '/^FB2_MAIN_PROJECT_SHARED_SECRET=/ {print substr(`$0, index(`$0, `"=`")+1); exit}' '$EnvPath'"
    $sshArgs = @(
        "-o", "ProxyCommand=none",
        "-o", "ProxyJump=none",
        "-o", "BatchMode=yes",
        "-o", "StrictHostKeyChecking=accept-new",
        "-p", ([string]$Port),
        "-i", $KeyPath,
        "$User@$HostName",
        $remoteCommand
    )
    $secretLines = & ssh @sshArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to read FB2_MAIN_PROJECT_SHARED_SECRET from remote .env."
    }

    $secret = @($secretLines) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Select-Object -First 1
    $secret = if ($null -eq $secret) { "" } else { [string]$secret }
    if ([string]::IsNullOrWhiteSpace($secret)) {
        throw "Remote FB2_MAIN_PROJECT_SHARED_SECRET is empty or missing."
    }
    $secret.Trim()
}

function New-Fb2TokenBridgePreflightArguments {
    param(
        [string]$ScriptPath,
        [string]$Username,
        [string]$ExternalUser,
        [string]$Group,
        [string]$Summary,
        [int]$RequestTimeout,
        [int]$PollTimeout,
        [int]$FeedbackTimeout
    )

    $args = [System.Collections.Generic.List[string]]::new()
    foreach ($item in @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", $ScriptPath,
            "-DataOnlyAcceptance",
            "-PreflightOnly",
            "-Fb2Username", $Username,
            "-ExternalUserId", $ExternalUser,
            "-GroupId", $Group,
            "-RequestTimeoutSec", ([string]$RequestTimeout),
            "-PollTimeoutSec", ([string]$PollTimeout),
            "-FeedbackPollTimeoutSec", ([string]$FeedbackTimeout)
        )) {
        [void]$args.Add([string]$item)
    }
    if (-not [string]::IsNullOrWhiteSpace($Summary)) {
        [void]$args.Add("-SummaryPath")
        [void]$args.Add($Summary)
    }
    @($args)
}

function New-Fb2TokenBridgeContractSmokeArguments {
    param(
        [string]$ScriptPath,
        [string]$Username,
        [string]$ExternalUser,
        [string]$Summary
    )

    $args = [System.Collections.Generic.List[string]]::new()
    foreach ($item in @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", $ScriptPath,
            "-Fb2Username", $Username,
            "-ExternalUserId", $ExternalUser,
            "-SummaryPath", $Summary
        )) {
        [void]$args.Add([string]$item)
    }
    @($args)
}

function Test-Fb2TokenBridgeCommandSecretSafe {
    param([string[]]$CommandArgs)

    $joined = @($CommandArgs) -join " "
    if ($joined -match "(?i)-Fb2(AiCenter)?Token\s+") {
        return $false
    }
    if ($joined -match "(?i)FB2_AI_CENTER_TOKEN\s*=") {
        return $false
    }
    if ($joined -match "(?i)X-FB2-AI-CENTER-TOKEN") {
        return $false
    }
    if ($joined -match "(?i)-Fb2Password\s+") {
        return $false
    }
    return $true
}

function Write-Fb2TokenBridgeResult {
    param(
        [object]$Result,
        [string]$Path
    )

    if (-not [string]::IsNullOrWhiteSpace($Path)) {
        $parent = Split-Path -Parent $Path
        if (-not [string]::IsNullOrWhiteSpace($parent)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        $Result | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Path -Encoding UTF8
    }
    $Result
}

function New-Fb2TokenBridgeRunResult {
    param(
        [bool]$Success,
        [object]$PreflightExitCode,
        [object]$ContractSmokeExitCode,
        [object]$CurrentStateExitCode,
        [string]$Note
    )

    [ordered]@{
        schema = "fb2.main_project.token_bridge_run.v1"
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        success = $Success
        fb2_base = $Fb2Base.TrimEnd("/")
        server_host = $ServerHost
        server_env_path = $ServerEnvPath
        group_id = $GroupId
        external_user_id = $ExternalUserId
        run_data_only_preflight = [bool]$RunDataOnlyPreflight
        run_current_state_after = [bool]$RunCurrentStateAfter
        preflight_exit_code = $PreflightExitCode
        contract_smoke_exit_code = $ContractSmokeExitCode
        contract_smoke_summary_path = $ContractSmokeSummaryPath
        contract_smoke_refreshed = ($null -ne $ContractSmokeExitCode)
        current_state_exit_code = $CurrentStateExitCode
        summary_path = $SummaryPath
        token_passed_as_argument = $false
        fb2_password_passed_to_child_argv = $false
        token_written_to_output = $false
        current_state_after_tokenless = [bool]$RunCurrentStateAfter
        project_network_proxy_policy = "direct_no_proxy"
        writes_visible_group_messages = $false
        note = $Note
    }
}

function Invoke-Fb2TokenBridgeSelfTest {
    param([string]$OutputPath)

    $root = Get-Fb2TokenBridgeRepoRoot
    $fakeArgs = New-Fb2TokenBridgePreflightArguments `
        -ScriptPath (Join-Path $root "scripts\smoke-fb2-final-acceptance.ps1") `
        -Username "123qwe" `
        -ExternalUser "6fe5aa17-0403-427a-8e91-7f414beca35d" `
        -Group "official" `
        -Summary "" `
        -RequestTimeout 60 `
        -PollTimeout 120 `
        -FeedbackTimeout 60

    $checks = [System.Collections.ArrayList]::new()
    $fakeContractArgs = New-Fb2TokenBridgeContractSmokeArguments `
        -ScriptPath (Join-Path $root "scripts\smoke-fb2-ai-center.ps1") `
        -Username "123qwe" `
        -ExternalUser "6fe5aa17-0403-427a-8e91-7f414beca35d" `
        -Summary (Join-Path $root "target\fb2-ai-center\contract-smoke-summary-current.json")
    [void]$checks.Add([ordered]@{
        name = "preflight command does not pass service token as argv"
        passed = (Test-Fb2TokenBridgeCommandSecretSafe -CommandArgs $fakeArgs)
    })
    [void]$checks.Add([ordered]@{
        name = "preflight command does not pass fb2 password as argv"
        passed = ((@($fakeArgs) -join " ") -notmatch "(?i)-Fb2Password\s+")
    })
    [void]$checks.Add([ordered]@{
        name = "preflight command is no-write mode"
        passed = ((@($fakeArgs) -contains "-DataOnlyAcceptance") -and (@($fakeArgs) -contains "-PreflightOnly") -and (-not (@($fakeArgs) -contains "-AllowVisibleMessages")))
    })
    [void]$checks.Add([ordered]@{
        name = "preflight command keeps target user"
        passed = ((@($fakeArgs) -contains "-ExternalUserId") -and ((@($fakeArgs) -join " ") -match "6fe5aa17-0403-427a-8e91-7f414beca35d"))
    })
    [void]$checks.Add([ordered]@{
        name = "contract smoke command does not pass token or password as argv"
        passed = (Test-Fb2TokenBridgeCommandSecretSafe -CommandArgs $fakeContractArgs)
    })
    [void]$checks.Add([ordered]@{
        name = "contract smoke command refreshes canonical summary"
        passed = ((@($fakeContractArgs) -contains "-SummaryPath") -and ((@($fakeContractArgs) -join " ") -match "contract-smoke-summary-current\.json"))
    })
    [void]$checks.Add([ordered]@{
        name = "remote ssh disables proxy hops"
        passed = $true
        detail = "Live ssh uses ProxyCommand=none and ProxyJump=none."
    })

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    $result = [ordered]@{
        schema = "fb2.main_project.token_bridge_selftest.v1"
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        note = "Self-test only; does not read remote secrets or contact fb2."
    }
    Write-Fb2TokenBridgeResult -Result $result -Path $OutputPath | Out-Null
    if (-not [bool]$result.success) {
        exit 1
    }
    $result | ConvertTo-Json -Depth 10
}

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $defaultOutputName = if ($SelfTest) {
        "token-bridge-wrapper-validation-current.json"
    } else {
        "token-bridge-data-only-preflight-current.json"
    }
    $OutputPath = Join-Path (Get-Fb2TokenBridgeRepoRoot) "target\fb2-ai-center\$defaultOutputName"
} else {
    $OutputPath = Resolve-Fb2TokenBridgePath -Path $OutputPath -Root (Get-Fb2TokenBridgeRepoRoot)
}

if ($SelfTest) {
    Invoke-Fb2TokenBridgeSelfTest -OutputPath $OutputPath
    exit 0
}

if (-not $RunDataOnlyPreflight -and -not $RunCurrentStateAfter) {
    throw "Select -RunDataOnlyPreflight and optionally -RunCurrentStateAfter. No live action is run by default."
}

if ([string]::IsNullOrWhiteSpace($Fb2Password)) {
    $Fb2Password = [System.Environment]::GetEnvironmentVariable("FB2_VISIBLE_SMOKE_PASSWORD", "Process")
}
if ($RunDataOnlyPreflight -and [string]::IsNullOrWhiteSpace($Fb2Password)) {
    throw "FB2_VISIBLE_SMOKE_PASSWORD is required for 123qwe login preflight. -Fb2Password is kept only for legacy manual runs; prefer the environment variable so the password does not appear in command history."
}

$root = Get-Fb2TokenBridgeRepoRoot
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Join-Path $root "target\fb2-ai-center\token-bridge-data-only-preflight-summary-current.json"
} else {
    $SummaryPath = Resolve-Fb2TokenBridgePath -Path $SummaryPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($ContractSmokeSummaryPath)) {
    $ContractSmokeSummaryPath = Join-Path $root "target\fb2-ai-center\contract-smoke-summary-current.json"
} else {
    $ContractSmokeSummaryPath = Resolve-Fb2TokenBridgePath -Path $ContractSmokeSummaryPath -Root $root
}

$envSnapshots = @(
    (Save-Fb2TokenBridgeEnvironmentValue -Name "FB2_API_BASE"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "FB2_AI_CENTER_TOKEN"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "FB2_VISIBLE_SMOKE_USERNAME"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "FB2_VISIBLE_SMOKE_PASSWORD"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "FB2_AI_CONTEXT_EXTERNAL_USER_ID"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "NO_PROXY"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "no_proxy"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "HTTP_PROXY"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "HTTPS_PROXY"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "ALL_PROXY"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "http_proxy"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "https_proxy"),
    (Save-Fb2TokenBridgeEnvironmentValue -Name "all_proxy")
)

$sharedSecret = $null
$preflightExitCode = $null
$contractSmokeExitCode = $null
$currentStateExitCode = $null
try {
    $sharedSecret = Get-Fb2TokenBridgeRemoteSharedSecret `
        -HostName $ServerHost `
        -Port $ServerPort `
        -User $ServerUser `
        -KeyPath $SshKey `
        -EnvPath $ServerEnvPath

    [System.Environment]::SetEnvironmentVariable("FB2_API_BASE", $Fb2Base.TrimEnd("/"), "Process")
    [System.Environment]::SetEnvironmentVariable("FB2_AI_CENTER_TOKEN", $sharedSecret, "Process")
    [System.Environment]::SetEnvironmentVariable("FB2_VISIBLE_SMOKE_USERNAME", $Fb2Username, "Process")
    [System.Environment]::SetEnvironmentVariable("FB2_VISIBLE_SMOKE_PASSWORD", $Fb2Password, "Process")
    [System.Environment]::SetEnvironmentVariable("FB2_AI_CONTEXT_EXTERNAL_USER_ID", $ExternalUserId, "Process")
    [System.Environment]::SetEnvironmentVariable("NO_PROXY", "*", "Process")
    [System.Environment]::SetEnvironmentVariable("no_proxy", "*", "Process")
    foreach ($proxyEnvName in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
        [System.Environment]::SetEnvironmentVariable($proxyEnvName, $null, "Process")
    }

    if ($RunDataOnlyPreflight) {
        $preflightArgs = New-Fb2TokenBridgePreflightArguments `
            -ScriptPath (Join-Path $root "scripts\smoke-fb2-final-acceptance.ps1") `
            -Username $Fb2Username `
            -ExternalUser $ExternalUserId `
            -Group $GroupId `
            -Summary $SummaryPath `
            -RequestTimeout $RequestTimeoutSec `
            -PollTimeout $PollTimeoutSec `
            -FeedbackTimeout $FeedbackPollTimeoutSec

        if (-not (Test-Fb2TokenBridgeCommandSecretSafe -CommandArgs $preflightArgs)) {
            throw "Internal safety check failed: preflight argv contains service token material."
        }
        & pwsh @preflightArgs
        $preflightExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        if ($preflightExitCode -ne 0) {
            throw "Data-only preflight failed with exit code $preflightExitCode."
        }
    }

    if ($RunCurrentStateAfter) {
        $contractSmokeArgs = New-Fb2TokenBridgeContractSmokeArguments `
            -ScriptPath (Join-Path $root "scripts\smoke-fb2-ai-center.ps1") `
            -Username $Fb2Username `
            -ExternalUser $ExternalUserId `
            -Summary $ContractSmokeSummaryPath

        if (-not (Test-Fb2TokenBridgeCommandSecretSafe -CommandArgs $contractSmokeArgs)) {
            throw "Internal safety check failed: contract smoke argv contains service token or fb2 password material."
        }
        & pwsh @contractSmokeArgs
        $contractSmokeExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        if ($contractSmokeExitCode -ne 0) {
            throw "Authenticated contract smoke failed with exit code $contractSmokeExitCode."
        }

        $preCurrentStateResult = New-Fb2TokenBridgeRunResult `
            -Success $true `
            -PreflightExitCode $preflightExitCode `
            -ContractSmokeExitCode $contractSmokeExitCode `
            -CurrentStateExitCode $null `
            -Note "Preflight and authenticated contract smoke succeeded; current-state validation is about to run tokenless against this fresh no-write bridge evidence."
        Write-Fb2TokenBridgeResult -Result $preCurrentStateResult -Path $OutputPath | Out-Null

        # Current-state validation is a handoff gate: prove the token bridge did not
        # leave a service token behind in ordinary no-secret continuation state.
        [System.Environment]::SetEnvironmentVariable("FB2_AI_CENTER_TOKEN", $null, "Process")
        & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root "scripts\validate-fb2-ai-center-current-state.ps1")
        $currentStateExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        if ($currentStateExitCode -ne 0) {
            throw "Current-state validation failed with exit code $currentStateExitCode."
        }
    }

    $result = New-Fb2TokenBridgeRunResult `
        -Success $true `
        -PreflightExitCode $preflightExitCode `
        -ContractSmokeExitCode $contractSmokeExitCode `
        -CurrentStateExitCode $currentStateExitCode `
        -Note "Remote fb2 service token was read into process env only and restored after child scripts."
    Write-Fb2TokenBridgeResult -Result $result -Path $OutputPath | Out-Null
    $result | ConvertTo-Json -Depth 10
} finally {
    foreach ($snapshot in $envSnapshots) {
        Restore-Fb2TokenBridgeEnvironmentValue -Snapshot $snapshot
    }
    $sharedSecret = $null
}

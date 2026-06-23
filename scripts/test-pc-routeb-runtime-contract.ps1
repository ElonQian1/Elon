param(
    [switch]$SkipServerRuntimeTests,
    [switch]$SkipPcDevRuntimeTests
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$ServerCargo = Join-Path $RepoRoot "server\Cargo.toml"
$PcDevRuntimeCargo = Join-Path $RepoRoot "server\pc-dev-runtime\Cargo.toml"
$ApiRuntimeConfig = Join-Path $RepoRoot "server\src\node_agent_api_runtime_config.rs"
$ApiRuntimeTools = Join-Path $RepoRoot "server\src\node_agent_api_runtime_tools.rs"
$ServerRuntime = Join-Path $RepoRoot "server\src\node_agent_server_runtime.rs"
$PcDevRuntime = Join-Path $RepoRoot "server\pc-dev-runtime\src\project_agent_runtime.rs"

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Body
    )

    Write-Host ""
    Write-Host "== $Label ==" -ForegroundColor Cyan
    & $Body
}

function Assert-FileContains {
    param(
        [string]$Path,
        [string]$Needle,
        [string]$Message
    )

    $text = Get-Content -LiteralPath $Path -Raw
    if (-not $text.Contains($Needle)) {
        throw $Message
    }
}

Set-Location $RepoRoot

Invoke-Step "Static Route B runtime contract" {
    foreach ($path in @($ApiRuntimeConfig, $ApiRuntimeTools, $ServerRuntime, $PcDevRuntime)) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing Route B runtime file: $path"
        }
    }

    foreach ($tool in @("list_dir", "read_file", "read_file_range", "write_file", "apply_patch", "run_command")) {
        Assert-FileContains `
            -Path $ApiRuntimeConfig `
            -Needle "`"$tool`"" `
            -Message "Route B tool contract no longer exposes $tool"
        Assert-FileContains `
            -Path $ApiRuntimeTools `
            -Needle "`"$tool`"" `
            -Message "Route B OpenAI tool definition no longer exposes $tool"
    }

    foreach ($tool in @("write_file", "apply_patch", "run_command")) {
        Assert-FileContains `
            -Path $ApiRuntimeConfig `
            -Needle "`"$tool`"" `
            -Message "Route B approval-required contract lost $tool"
    }

    Assert-FileContains `
        -Path $ApiRuntimeConfig `
        -Needle "workspace_relative_no_git_no_symlink_escape" `
        -Message "Route B path policy must remain workspace-scoped and deny .git/symlink escape"
    Assert-FileContains `
        -Path $ApiRuntimeConfig `
        -Needle "structured_project_command_allowlist" `
        -Message "Route B command policy must remain structured allowlist based"
    Assert-FileContains `
        -Path $ApiRuntimeConfig `
        -Needle "write_file_apply_patch_run_command_require_user_approval" `
        -Message "Route B write/patch/command approval policy is missing"
    Assert-FileContains `
        -Path $ApiRuntimeConfig `
        -Needle "without_original_tty_reattach" `
        -Message "Route B recovery policy must keep the original CLI TTY limitation explicit"

    Assert-FileContains `
        -Path $ApiRuntimeTools `
        -Needle '"tool_choice"] = json!("auto")' `
        -Message "Route B API payload should prefer native tool/function calling"
    Assert-FileContains `
        -Path $ApiRuntimeTools `
        -Needle '"additionalProperties": false' `
        -Message "Route B tool schemas should reject undeclared properties"
    Assert-FileContains `
        -Path $ServerRuntime `
        -Needle "should_retry_without_tools" `
        -Message "Route B must keep compatibility fallback for providers without tools"
    Assert-FileContains `
        -Path $ServerRuntime `
        -Needle "runtime_http_error_message" `
        -Message "Route B provider error bodies must stay redacted"
    Assert-FileContains `
        -Path $ServerRuntime `
        -Needle "limited_runtime_response_text" `
        -Message "Route B provider response body reads must stay bounded"

    Assert-FileContains `
        -Path $PcDevRuntime `
        -Needle "Route B is intentionally conservative" `
        -Message "Generated pc-dev runtime docs must keep the Route B safety warning"
    Assert-FileContains `
        -Path $PcDevRuntime `
        -Needle "-DryRun" `
        -Message "Generated pc-dev runtime must keep dry-run support"
    Assert-FileContains `
        -Path $PcDevRuntime `
        -Needle "-MaxRunCommands" `
        -Message "Generated pc-dev runtime must keep command budget support"
    Write-Host "Static Route B runtime contract passed."
}

if (-not $SkipServerRuntimeTests) {
    Invoke-Step "Route B node-agent runtime unit tests" {
        cargo test --manifest-path $ServerCargo node_agent_api_runtime_config -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "node_agent_api_runtime_config tests failed with exit code $LASTEXITCODE"
        }

        cargo test --manifest-path $ServerCargo node_agent_api_runtime_tools -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "node_agent_api_runtime_tools tests failed with exit code $LASTEXITCODE"
        }

        cargo test --manifest-path $ServerCargo api_runtime_ -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "node_agent_server_runtime api_runtime tests failed with exit code $LASTEXITCODE"
        }
    }
}

if (-not $SkipPcDevRuntimeTests) {
    Invoke-Step "Route B pc-dev-runtime generated script tests" {
        cargo test --manifest-path $PcDevRuntimeCargo agent_runtime -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "pc-dev-runtime agent_runtime tests failed with exit code $LASTEXITCODE"
        }
    }
}

Write-Host ""
Write-Host "PC Route B runtime contract gate passed." -ForegroundColor Green

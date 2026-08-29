$ErrorActionPreference = 'Stop'

$TaskRepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$FrontendDirectory = Join-Path $TaskRepoRoot 'pc-frontend'
$RustManifest = Join-Path $TaskRepoRoot 'desktop-shell\src-tauri\Cargo.toml'

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )
    Write-Host "VALIDATION_STEP=$Label"
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

Invoke-Checked 'adapter_js_syntax' {
    $AdapterAssets = @(
        Get-ChildItem (Join-Path $TaskRepoRoot 'android\app\src\main\assets\*') `
            -Include 'chatgpt_web_adapter*.js','google_web_*.js' -File
        Get-ChildItem (Join-Path $TaskRepoRoot 'desktop-shell\src-tauri\src\local_ai_browser\*adapter.js') -File
    )
    if (-not $AdapterAssets.Count) { throw 'No shared Web AI adapter assets were found.' }
    foreach ($Asset in $AdapterAssets) {
        & node --check $Asset.FullName
        if ($LASTEXITCODE -ne 0) { throw "JavaScript syntax failed: $($Asset.Name)" }
    }
}
Invoke-Checked 'win_response_research_capture' {
    & node (Join-Path $TaskRepoRoot 'scripts\test-win-web-response-research-capture.js')
}
Invoke-Checked 'google_win_private_fetch_tap' {
    & node --check (Join-Path $TaskRepoRoot 'desktop-shell\src-tauri\src\local_ai_browser\google_win_private_fetch_tap.js')
    if ($LASTEXITCODE -ne 0) { return }
    & node (Join-Path $TaskRepoRoot 'scripts\test-google-win-private-fetch-tap.js')
}
Invoke-Checked 'win_private_stream_binding' {
    & node (Join-Path $TaskRepoRoot 'scripts\test-chatgpt-win-private-stream-binding.js')
}
Invoke-Checked 'win_private_guest_conversation_transport' {
    & node (Join-Path $TaskRepoRoot 'scripts\test-chatgpt-win-private-guest-conversation-transport.js')
}
Invoke-Checked 'win_private_transport_health' {
    & node (Join-Path $TaskRepoRoot 'scripts\test-chatgpt-win-private-transport-health.js')
}
Invoke-Checked 'pc_user_browser_contracts' { & npm.cmd --prefix $FrontendDirectory run test:user-browser }
Invoke-Checked 'pc_typecheck_and_vite_build' { & npm.cmd --prefix $FrontendDirectory run build }
Invoke-Checked 'pc_eslint' { & npm.cmd --prefix $FrontendDirectory run lint }
Invoke-Checked 'tauri_local_ai_rust_tests' {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'validate-rust.ps1') `
        -Domain 'local-ai-browser' -- test --manifest-path $RustManifest --bin elon-desktop local_ai_browser
}

Write-Host 'WIN_WEB_AI_VALIDATION=passed'

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'mobile-pwa-runtime-template.ps1')
$repoRoot = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $repoRoot 'server/src/assets'
$output = Join-Path $repoRoot ".ai-tmp/mobile-pwa-runtime-test-$PID.html"
try {
    $result = New-ElonMobilePwaRuntimeTemplate -TemplatePath (Join-Path $assets 'web_page.html') `
        -StylesPath (Join-Path $assets 'project_plaza.css') `
        -ThemeStylesPath (Join-Path $assets 'orbital_mobile_theme.css') `
        -CacheScriptPath (Join-Path $assets 'project_plaza_cache.js') `
        -ScriptPath (Join-Path $assets 'project_plaza.js') -OutputPath $output
    $html = [IO.File]::ReadAllText($result.FullName)
    if ($html -notmatch '<script type="module" data-elon-runtime-asset="/assets/scan_pwa.js">') {
        throw 'Scanner lost ES-module execution in the served runtime template.'
    }
    if ($html -notmatch '<script data-elon-runtime-asset="/assets/social_links.js">') {
        throw 'Classic startup scripts changed execution type.'
    }
    $sharedCss = [IO.File]::ReadAllText((Join-Path $repoRoot 'shared/scan/browserScanDialog.css'))
    if (-not $html.Contains($sharedCss)) { throw 'Scanner shared stylesheet was not embedded.' }
    $scannerJs = [IO.File]::ReadAllText((Join-Path $assets 'scan_pwa.js'))
    if (-not $html.Contains($scannerJs)) { throw 'Scanner module was not embedded intact.' }
    if ($html -notmatch '<button[^>]+class="add-friend-scan-pill"') {
        throw 'Friend scanner entry is missing from the published document.'
    }
    Write-Host 'MOBILE_PWA_RUNTIME_SCANNER=passed checks=5'
} finally {
    if (Test-Path -LiteralPath $output) { Remove-Item -LiteralPath $output }
}

#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "../android/gradle/chatgpt-web-input-fingerprint.gradle"
$source = Get-Content -LiteralPath $path -Raw

foreach ($required in @(
    "def normalizeFingerprintText",
    'text.replace("\r\n", "\n").replace("\r", "\n")',
    "def fingerprintText",
    "def composerDiscoveryCaseInputs",
    'ChatGptWebDiscoveryEvidence.kt',
    'composerCaseInputs + composerDiscoveryCaseInputs',
    "def acceptanceCaseIds",
    "ChatGPT Web verification fingerprint catalog mismatch",
    "def verificationCaseContractRevisionOverrides",
    "def verificationCaseContractRevisions",
    "chatGptWebVerificationCaseContractRevisionJson",
    "CHATGPT_WEB_VERIFICATION_CONTRACT_REVISION=",
    'verificationCaseFiles["safe/single_webview_skin"]',
    '["projects", "tasks", "images", "library", "gpts", "apps", "work"]',
    "def normalizedInputBytes",
    "digest.update(normalizedInputBytes(input))",
    'tasks.register("verifyChatGptWebFingerprintPortability")',
    'lfText.replace("\n", "\r\n")',
    'lfText.replace("\n", "\r")',
    'tasks.matching { it.name == "preBuild" }.configureEach',
    'dependsOn("verifyChatGptWebFingerprintPortability")',
    "CHATGPT_WEB_FINGERPRINT_PORTABILITY=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT Web fingerprint portability contract is missing: $required"
    }
}

if ($source.Contains("digest.update(input.bytes)")) {
    throw "ChatGPT Web global fingerprint still hashes checkout-specific raw bytes."
}
if ($source.Contains('scripts/chatgpt-web-smoke-*.ps1')) {
    throw "ChatGPT Web case fingerprints still invalidate every case for a local helper change."
}
foreach ($forbidden in @(
    'scripts/chatgpt-web-smoke-runtime.ps1',
    'scripts/chatgpt-web-smoke-evidence.ps1',
    'scripts/chatgpt-web-smoke-composer.ps1',
    'scripts/chatgpt-web-smoke-supervised-runtime.ps1',
    'scripts/chatgpt-web-smoke-conversation-sample.ps1',
    'filesMatching(["scripts/${smokeScript}"])'
)) {
    if ($source.Contains($forbidden)) {
        throw "Acceptance harness implementation leaked into product behavior fingerprints: $forbidden"
    }
}

$buildSource = Get-Content (Join-Path $PSScriptRoot "../android/app/build.gradle") -Raw
if (-not $buildSource.Contains(
    'CHATGPT_WEB_VERIFICATION_CASE_CONTRACT_REVISION_JSON'
)) {
    throw "BuildConfig is missing ChatGPT Web verification contract revisions."
}

Write-Output "CHATGPT_WEB_INPUT_FINGERPRINT_CONTRACT=passed"

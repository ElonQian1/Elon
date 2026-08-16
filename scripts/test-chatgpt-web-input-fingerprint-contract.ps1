#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "../android/gradle/chatgpt-web-input-fingerprint.gradle"
$source = Get-Content -LiteralPath $path -Raw

foreach ($required in @(
    "def normalizeFingerprintText",
    'text.replace("\r\n", "\n").replace("\r", "\n")',
    "def fingerprintText",
    "def supervisedSmokeCaseInputs",
    'scripts/chatgpt-web-smoke-supervised-runtime.ps1',
    "def conversationSampleCaseInputs",
    'scripts/chatgpt-web-smoke-conversation-sample.ps1',
    'supervisedSmokeCaseInputs + conversationSampleCaseInputs',
    "def composerDiscoveryCaseInputs",
    'ChatGptWebDiscoveryEvidence.kt',
    'composerCaseInputs + composerDiscoveryCaseInputs',
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

foreach ($caseId in @(
    "safe/message_actions",
    "supervised/message_actions",
    "safe/conversation_management_structure",
    "supervised/conversation_mutations"
)) {
    $pattern = 'verificationCaseFiles\["' + [regex]::Escape($caseId) +
        '"\]\s*=\s*caseInputs\([^\)]*supervisedSmokeCaseInputs \+ ' +
        'conversationSampleCaseInputs'
    if ($source -notmatch $pattern) {
        throw "Conversation sample fingerprint scope is missing for case: $caseId"
    }
}

Write-Output "CHATGPT_WEB_INPUT_FINGERPRINT_CONTRACT=passed"

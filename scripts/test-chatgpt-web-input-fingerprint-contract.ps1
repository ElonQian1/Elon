#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "../android/gradle/chatgpt-web-input-fingerprint.gradle"
$source = Get-Content -LiteralPath $path -Raw

foreach ($required in @(
    "def normalizeFingerprintText",
    'text.replace("\r\n", "\n").replace("\r", "\n")',
    "def fingerprintText",
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

Write-Output "CHATGPT_WEB_INPUT_FINGERPRINT_CONTRACT=passed"

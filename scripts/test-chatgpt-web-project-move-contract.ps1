#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-project-move.ps1"
$source = Get-Content -LiteralPath $path -Raw

function Assert-Contains([string]$Needle) {
    if (-not $source.Contains($Needle)) { throw "Missing project-move smoke contract: $Needle" }
}

@(
    "[switch]`$ConfirmRoundTrip",
    "Wait-ProductionReady",
    "Open-ChatGptWebNativeChatSurface",
    "get_web_chat_navigation",
    "ConvertTo-NativeToken",
    '-Stage "conversation-actions"',
    '-Stage "move-action"',
    '-Stage "project-destination"',
    "chatgpt-conversation-actions:",
    "-Prefix",
    "-Optional",
    'for ($attempt = 1; $attempt -le 2; $attempt++)',
    "one safe retry",
    "web-chat-conversation-action-move-to-project",
    "web-chat-conversation-project-destination:",
    "Wait-ConversationMembership",
    "Test-ProjectMoveWriteObserved",
    'text -eq "正在提交一次移动操作"',
    'text -eq "正在同步会话目录"',
    "[ref]`$WriteSelected",
    "-not `$restoreWriteSelected",
    "original_membership_restored",
    "private_content_emitted = `$false",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "Project-move recovery is ambiguous",
    "if (`$null -ne `$primaryFailure) { throw `$primaryFailure }",
    "if (`$null -ne `$cleanupFailure) { throw `$cleanupFailure }",
    "CHATGPT_WEB_PROJECT_MOVE_STATUS=passed"
) | ForEach-Object { Assert-Contains $_ }

$guardStart = $source.IndexOf("if (`$ConfirmRoundTrip)")
$guardEnd = $source.IndexOf("Close-Sidebar", $guardStart)
if ($guardStart -lt 0 -or $guardEnd -le $guardStart) {
    throw "Round-trip confirmation guard could not be inspected."
}
$guardedWrite = $source.Substring($guardStart, $guardEnd - $guardStart)
if (-not $guardedWrite.Contains("Invoke-ProjectMove")) {
    throw "Round-trip write must remain behind explicit confirmation."
}

Write-Output "CHATGPT_WEB_PROJECT_MOVE_CONTRACT=passed"

#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-project-move.ps1"
$observationPath = Join-Path $PSScriptRoot "chatgpt-web-project-move-write-observation.ps1"
$source = (Get-Content -LiteralPath $path -Raw) + "`n" +
    (Get-Content -LiteralPath $observationPath -Raw)

function Assert-Contains([string]$Needle) {
    if (-not $source.Contains($Needle)) { throw "Missing project-move smoke contract: $Needle" }
}

@(
    "[switch]`$ConfirmRoundTrip",
    "[ValidateRange(30, 180)][int]`$TimeoutSec = 150",
    "[ValidateRange(0, 39)][int]`$TargetProjectOffset = 0",
    "Wait-ProductionReady",
    "Ensure-ProductionReady",
    "Invoke-ReadActionWithSurfaceRecovery",
    'web_chat_mode_inactive|web_chat_not_ready|main_activity_not_bound',
    "The project sidebar did not settle after its background refresh.",
    "expectedByToken",
    "Open-ChatGptWebNativeChatSurface",
    "get_web_chat_navigation",
    "ConvertTo-NativeToken",
    "Get-ConversationProjectIdFromPath",
    "Get-CanonicalConversationMembership",
    "Get-LiveConversationMembership",
    "Wait-ReadOnlyOriginalMembership",
    "Select-OfficialFallbackProject",
    "Get-ConversationProjectIdFromPath -Path ([string]`$_.path)",
    '$orderedProjects = @($Navigation.projects | Sort-Object',
    '$expectedByToken[[string]$_].active -eq $true',
    '-Stage "conversation-actions"',
    '-Stage "move-action"',
    '-Stage "project-destination"',
    "chatgpt-conversation-actions:",
    "-Prefix",
    "-Optional",
    "Close-Sidebar",
    'for ($attempt = 1; $attempt -le 2; $attempt++)',
    "one safe retry",
    'Select-Object -Skip $TargetProjectOffset -First 1',
    "web-chat-conversation-action-move-to-project",
    '$visibleToUser -ne "false"',
    '"tap", "$x", "$y"',
    '/data/local/tmp/elon-chatgpt-project-move-$PID.xml',
    'for ($attempt = 1; $attempt -le 3; $attempt++)',
    "web-chat-conversation-project-destination:",
    "Wait-ConversationMembership",
    "Restore-WebChatNativeConversation -Runtime `$runtime",
    "The moved conversation did not reopen before the restore operation.",
    "The moved conversation did not reopen for cleanup.",
    "Test-ProjectMoveWriteObserved",
    "chatgpt-web-project-move-write-observation.ps1",
    "web_chat_project_move_reconciliation",
    "since_wall_time_ms",
    "SinceWallTimeMs",
    "Test-ProjectMoveWriteObserved -SinceWallTimeMs",
    "Get-ProjectMoveUiStage",
    "Dismiss-StalePreWriteMoveFailure",
    "dismiss stale pre-write project-move failure",
    'Write-Host "CHATGPT_WEB_PROJECT_MOVE_PROGRESS=$uiStage"',
    '"failed_before_write"',
    '"failed_after_write"',
    '"正在提交一次移动操作" -in $texts',
    '"正在同步会话目录" -in $texts',
    "[ref]`$WriteSelected",
    "[ref]`$DestinationProject",
    "[switch]`$AllowFallbackDestination",
    "CHATGPT_WEB_PROJECT_MOVE_PROGRESS=official_destination_selected",
    "-not `$restoreWriteSelected",
    "original_membership_restored",
    "CHATGPT_WEB_PROJECT_MOVE_RECOVERY=",
    "cleanup_write_selected=",
    "recovery_unknown=",
    "private_content_emitted = `$false",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "Project-move recovery is ambiguous",
    "if (`$null -ne `$primaryFailure) { throw `$primaryFailure }",
    "if (`$null -ne `$cleanupFailure) { throw `$cleanupFailure }",
    "CHATGPT_WEB_PROJECT_MOVE_STATUS=passed"
) | ForEach-Object { Assert-Contains $_ }

$mainActionStart = $source.IndexOf("function Invoke-MainAction")
$mainActionEnd = $source.IndexOf("function Wait-ProductionReady", $mainActionStart)
if ($mainActionStart -lt 0 -or $mainActionEnd -le $mainActionStart) {
    throw "Invoke-MainAction could not be inspected."
}
$mainAction = $source.Substring($mainActionStart, $mainActionEnd - $mainActionStart)
if ($mainAction.Contains("EnsureMainActivity")) {
    throw "Routine project-move actions must not restart MainActivity."
}

${backAction} = '@("shell", "input", "keyevent", "4")'
${backMatches} = [regex]::Matches(${source}, [regex]::Escape(${backAction}))
${dismissStart} = ${source}.IndexOf("function Dismiss-StalePreWriteMoveFailure")
${dismissEnd} = ${source}.IndexOf("function Wait-ConversationMembership", ${dismissStart})
if (${backMatches}.Count -ne 1 -or ${dismissStart} -lt 0 -or ${dismissEnd} -le ${dismissStart}) {
    throw "Android Back must be limited to one guarded stale pre-write dialog dismissal."
}
${dismissGuard} = ${source}.Substring(${dismissStart}, ${dismissEnd} - ${dismissStart})
if (
    -not ${dismissGuard}.Contains('Get-ProjectMoveUiStage) -ne "failed_before_write"') -or
    -not ${dismissGuard}.Contains(${backAction})
) {
    throw "Android Back must remain guarded by the pre-write failure stage."
}

$guardStart = $source.IndexOf("if (`$ConfirmRoundTrip)")
$guardEnd = $source.IndexOf("Close-Sidebar", $guardStart)
if ($guardStart -lt 0 -or $guardEnd -le $guardStart) {
    throw "Round-trip confirmation guard could not be inspected."
}
$guardedWrite = $source.Substring($guardStart, $guardEnd - $guardStart)
if (-not $guardedWrite.Contains("Invoke-ProjectMove")) {
    throw "Round-trip write must remain behind explicit confirmation."
}
if (-not $guardedWrite.Contains("-AllowFallbackDestination")) {
    throw "Only the forward reversible move may choose an officially eligible fallback project."
}
$restoreStart = $guardedWrite.IndexOf('$restoreDestination = $originProject')
if ($restoreStart -lt 0) { throw "Original-project restoration could not be inspected." }
$restoreBlock = $guardedWrite.Substring($restoreStart)
if ($restoreBlock.Contains("-AllowFallbackDestination")) {
    throw "Original-project restoration must never substitute another destination."
}

Write-Output "CHATGPT_WEB_PROJECT_MOVE_CONTRACT=passed"

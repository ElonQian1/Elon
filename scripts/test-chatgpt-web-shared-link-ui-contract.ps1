#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-shared-link-ui.ps1') -Raw
$java = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/SharedLinkUiAcceptance.java') -Raw
$tokens = $null; $errors = $null
[Management.Automation.Language.Parser]::ParseInput($source, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count) { throw 'shared_link_script_syntax' }
foreach ($guard in @('fixture_content_not_safe_to_publish','new_fixture_already_shared','created_link_was_preexisting',
    'new_share_not_first_in_confirmed_list','native_account_list_unconfirmed',
    '$page.items[0].id -cne $shareId','$page.items[0].path -cne $probe',
    '$revokeDispatched = $true', '-not $revokeDispatched',
    'verify_unshared_fixture','throw ([string]$failed.result.detail)',
    '-ExpectedAction open_conversation','live sharing fixture navigation',
    'catch { $report.cleanup_pending = $true }','catch { $report.menu_restoration_failed = $true }',
    'other_links_unchanged','Restore-WebChatNativeConversation','Stop-ChatGptWebSmokeAwakeLease')) {
    if (-not $source.Contains($guard)) { throw "missing_sharing_guard:$guard" }
}
foreach ($guard in @('assertSelected(expected)','revoke_confirmation_target_mismatch',
    'AccessibilityNodeInfo.ACTION_CLICK','AccessibilityNodeInfo.ACTION_PASTE','copied_share_mismatch',
    'clear_test_draft_failed','foreground_package_mismatch','content_exported')) {
    if (-not $java.Contains($guard)) { throw "missing_native_sharing_guard:$guard" }
}
if (-not $java.Contains('first_share_row_not_selected') -or -not $java.Contains('assertSelected(expected);')) {
    throw 'unguarded_share_keyboard_selection'
}
foreach ($forbidden in @('getUiDevice().click(', 'dumpWindowHierarchy', 'pressDelete', 'clearPrimaryClip')) {
    if ($java.Contains($forbidden)) { throw 'unsafe_native_sharing_automation' }
}
if ($java -match '\.put\("(?:url|text|content|clipboard)",') { throw 'native_private_content_export' }
'SHARED_LINK_UI_CONTRACT=passed'

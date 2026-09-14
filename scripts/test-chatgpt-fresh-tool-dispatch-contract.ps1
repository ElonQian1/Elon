#requires -Version 7.0
$ErrorActionPreference='Stop'
$path=Join-Path $PSScriptRoot 'smoke-chatgpt-fresh-tool-dispatch.ps1'
$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count){throw 'fresh_tool_script_parse_failed'}
$source=Get-Content -LiteralPath $path -Raw
$checks=0
function Before([string]$First,[string]$Second) {
    $a=$source.IndexOf($First,[StringComparison]::Ordinal)
    $b=$source.IndexOf($Second,[StringComparison]::Ordinal)
    if($a -lt 0 -or $b -le $a){throw 'fresh_tool_order_contract_failed'}
    $script:checks++
}
Before 'Assert-ChatGptWebSmokeTrustedDevice' 'Start-ChatGptWebSmokeAwakeLease'
Before 'Assert-OwnedFixture $web $main' 'Stage tool_context_preparation'
Before "if (`$PreflightOnly) {Stage preflight_complete}" 'Stage native_tool_selection'
Before 'Ui prepare_extended_tool_fixture' '$before=Trial start'
Before '        Save-Ledger' 'Ui send_extended_tool_fixture'
Before '$awaiting=$true' 'Ui send_extended_tool_fixture'
Before 'Ui send_extended_tool_fixture' '$report.native_click_acknowledged=$true'
Before 'if($awaiting){$report.write_unconfirmed=$true' 'if($trialRequested -and'
foreach($required in @('tool_fixture_requires_readonly_recovery','scope_already_verified_do_not_repeat',
    '[IO.FileShare]::None','Test-ChatGptFreshSendEvidence','Get-ChatGptFreshToolNativeEvidence',
    "`$web.conversation.url -cne ('https://chatgpt.com'+`$seed.resolved_path)",
    'foreground_changed_skip_restore','replay_allowed=$false','Where-Object {$null -ne $_}')) {
    if(!$source.Contains($required)){throw 'fresh_tool_safety_contract_missing'}
    $checks++
}
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/ConversationUiAcceptance.java') -Raw
$start=$java.IndexOf('case "prepare_extended_tool_fixture":',[StringComparison]::Ordinal)
$end=$java.IndexOf('case "send_extended_tool_fixture":',[StringComparison]::Ordinal)
if($start -lt 0 -or $end -le $start){throw 'fresh_tool_native_prepare_missing'}
$prepare=$java.Substring($start,$end-$start)
foreach($required in @('fixture_preview_ambiguous','ELON_EXTENDED_TOOL_ACCEPTANCE_V1 ',
    'fixture_input_not_open','fixture_prompt_missing')) {
    if(!$prepare.Contains($required)){throw 'fresh_tool_native_prepare_guard_missing'}
    $checks++
}
if($prepare.Contains('web-chat-send')){throw 'fresh_tool_prepare_must_not_send'}
$checks++
Write-Output "FRESH_TOOL_DISPATCH_CONTRACT=passed checks=$checks"

#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-stopped-turn-evidence.ps1')
$partial = 'ANSWER-START ' + ('synthetic partial answer ' * 5)
$argsForCase = @{ FirstPrompt = 'first'; SecondPrompt = 'second'; PartialReply = $partial; ExpectedReply = 'OK' }
$state = @{ social_chat = @{ web_chat_streaming = $false; messages = @(
    @{ role = 'user'; content = 'first' }, @{ role = 'friend'; content = $partial + ' tail' },
    @{ role = 'user'; content = 'second' }, @{ role = 'friend'; content = 'OK.' }
) } }
if (-not (Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'valid continuity rejected' }
$state.social_chat.web_chat_streaming = $true
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'unfinished stream accepted' }
$state.social_chat.web_chat_streaming = $false
$state.social_chat.messages[1].content = 'replacement'
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'lost partial accepted' }
$state.social_chat.messages = @(@{ role = 'user'; content = "first`n`nsecond" }, @{ role = 'friend'; content = 'OK' })
$merged = Get-ChatGptStoppedTurnEvidence -State $state @argsForCase
if ($merged.passed -or $merged.prompts_separate -or $merged.stopped_reply_retained) { throw 'merged turns accepted' }
$state.social_chat.messages = @()
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'empty projection accepted' }
$tokens = $null; $errors = $null
[Management.Automation.Language.Parser]::ParseFile(
    (Join-Path $PSScriptRoot 'smoke-chatgpt-web-stopped-followup.ps1'), [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count) { throw 'smoke script syntax failed' }
Write-Output 'STOPPED_TURN_EVIDENCE_TESTS=passed:6'

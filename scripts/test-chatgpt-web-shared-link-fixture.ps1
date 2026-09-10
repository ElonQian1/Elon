#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-shared-link-fixture.ps1')
$marker = 'ELON-SHARE-UI-0123456789abcdef0123456789abcdef'
$prompt = 'Reply only with this exact test marker: ' + $marker
$url = 'https://chatgpt.com/c/11111111-1111-4111-8111-111111111111'
function Context {
    @{conversation_url=$url;context_complete=$true;context_streaming=$false;has_more=$false;
        has_more_before=$false;message_offset=0;message_count=2;messages=@(
        @{role='user';state='completed';content=$prompt;content_truncated=$false;parts=@();parts_truncated=$false},
        @{role='assistant';state='completed';content=$marker;content_truncated=$false;parts=@();parts_truncated=$false})}
}
function Check($context, [bool]$expected, [string]$name) {
    if ((Test-ChatGptSharedLinkFixture -Context $context -ExpectedUrl $url -Prompt $prompt -Marker $marker) -ne $expected) { throw $name }
}
Check (Context) $true 'ordinary_fixture'
$c=Context; $c.messages += @{role='assistant';state='completed';content='Done';content_truncated=$false;parts=@();parts_truncated=$false}
$c.message_count=3
Check $c $true 'multiple_assistant_rows'
foreach ($mutation in @(
    {param($c) $c.conversation_url += '?different'},
    {param($c) $c.context_complete = $false},
    {param($c) $c.context_streaming = $true},
    {param($c) $c.has_more = $true},
    {param($c) $c.has_more_before = $true},
    {param($c) $c.message_offset = 1},
    {param($c) $c.message_count = 12},
    {param($c) $c.messages[0].content = 'private user draft'},
    {param($c) $c.messages[1].content = 'different reply'},
    {param($c) $c.messages[1].role = 'user'},
    {param($c) $c.messages[1].role = 'tool'},
    {param($c) $c.messages[1].state = 'streaming'},
    {param($c) $c.messages[1].content_truncated = $true},
    {param($c) $c.messages[1].parts_truncated = $true},
    {param($c) $c.messages[1].parts = @(@{type='image'})},
    {param($c) $c.messages[1].content = $marker + ('x' * 300)},
    {param($c) $c.messages += $c.messages[1],$c.messages[1],$c.messages[1]},
    {param($c) $c.messages = @($c.messages[0])}
)) {
    $c=Context; & $mutation $c; Check $c $false 'unsafe_fixture_accepted'
}
'SHARED_LINK_FIXTURE_TESTS=passed cases=20'

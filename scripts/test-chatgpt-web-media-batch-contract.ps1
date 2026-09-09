#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot 'smoke-chatgpt-web-media-batch.ps1'
$tokens = $null; $errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Media smoke parse failure' }
$source = $ast.Extent.Text
$check = $ast.Find({param($n)
    $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Check-Reply'
}, $true)
. ([scriptblock]::Create($check.Extent.Text))
$full = 'ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready ELON_PRIVATE_PDF_FIXTURE_V1=ready Three blue squares and one red circle.'
$good = Check-Reply $full
if (-not $good.text_read -or -not $good.pdf_read -or -not $good.image_read) { throw 'All facts required' }
foreach ($case in @(
    @{ text = $full.Replace('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready', 'unavailable'); missing = 'text_read' },
    @{ text = $full.Replace('ELON_PRIVATE_PDF_FIXTURE_V1=ready', 'unavailable'); missing = 'pdf_read' },
    @{ text = $full.Replace('Three blue squares', 'Two blue squares'); missing = 'image_read' },
    @{ text = $full.Replace('red circle', 'green circle'); missing = 'image_read' }
)) {
    if ((Check-Reply $case.text)[$case.missing]) { throw 'Missing attachment facts accepted' }
}
$prompt = $ast.Find({param($n)
    $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -eq '$prompt'
}, $true).Right.Extent.Text
if ($prompt -match 'ELON_|blue|red|square|circle') { throw 'Prompt leaks fixture answers' }
if ([regex]::Matches($source, "Act 'send_input'").Count -ne 1) { throw 'Only one message dispatch allowed' }
foreach ($required in @('Assert-ChatGptWebSmokeTrustedDevice', 'blank_idle_chat_required',
    'active_transaction_preserved', 'changed_draft_preserved', 'private_attachment_associated',
    'official_runtime_v1:accepted', 'Stop-ChatGptWebSmokeAwakeLease', 'synthetic_remote_artifacts_may_remain')) {
    if (-not $source.Contains($required)) { throw "Missing media smoke guard: $required" }
}
foreach ($forbidden in @('pm clear', 'removeAllCookies', 'uiautomator', 'input tap', 'DCIM', 'delete_conversation')) {
    if ($source.Contains($forbidden)) { throw "Unsafe media smoke operation: $forbidden" }
}
foreach ($name in @('Get-MediaProjectId', 'Test-FreshProjectReceipt', 'Get-MediaConversationId')) {
    $fn = $ast.Find({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq $name}.GetNewClosure(), $true)
    . ([scriptblock]::Create($fn.Extent.Text))
}
$id = 'g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
if ((Get-MediaProjectId ([uri]'https://chatgpt.com/') 'ordinary_new') -ne '') { throw 'Ordinary scope changed' }
if ((Get-MediaProjectId ([uri]"https://chatgpt.com/g/$id-test/project") 'project_new') -cne $id) { throw 'Project scope missing' }
foreach ($url in @('https://chatgpt.com/', "https://chatgpt.com/g/$id/c/11111111-2222-3333-4444-555555555555",
    "https://chatgpt.com/g/$id/project?temporary-chat=true", "https://chatgpt.com/g/$id/project#fragment",
    "https://chatgpt.com:8443/g/$id/project", "https://other.test/g/$id/project", "https://user@chatgpt.com/g/$id/project")) {
    $rejected = $false
    try { Get-MediaProjectId ([uri]$url) 'project_new' | Out-Null } catch { $rejected=$true }
    if (-not $rejected) { throw 'Foreign or nonblank project scope accepted' }
}
$receipt = @{action='probe_conversation_project';ok=$true;observed_at_ms=100}
if (-not (Test-FreshProjectReceipt $receipt 100)) { throw 'Fresh membership receipt rejected' }
if (Test-FreshProjectReceipt $receipt 101) { throw 'Stale membership receipt accepted' }
$receipt.ok=$false
if (Test-FreshProjectReceipt $receipt 100) { throw 'Unconfirmed membership accepted' }
$conversation = '11111111-2222-3333-4444-555555555555'
foreach ($path in @("/c/$conversation", "/g/$id/c/$conversation", "/g/$id-test/c/$conversation")) {
    if ((Get-MediaConversationId $path $id) -cne $conversation) { throw 'Same-thread route alias rejected' }
}
if (Get-MediaConversationId "/g/g-p-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/c/$conversation" $id) { throw 'Foreign project accepted' }
foreach ($required in @('Confirm-ProjectMembership', 'conversation_changed', 'project_membership_unconfirmed', 'project_path=$originalPath')) {
    if (-not $source.Contains($required)) { throw "Project acceptance guard missing: $required" }
}
'MEDIA_BATCH_SMOKE_CONTRACT=passed'

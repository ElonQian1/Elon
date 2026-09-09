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
'MEDIA_BATCH_SMOKE_CONTRACT=passed'

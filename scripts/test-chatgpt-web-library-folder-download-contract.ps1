#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-library-folder-download.ps1') -Raw
$java = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/LibraryUiAcceptance.java') -Raw
$tokens = $null; $errors = $null
[Management.Automation.Language.Parser]::ParseInput($source, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count) { throw 'folder_download_syntax' }
foreach ($guard in @('AllowExistingPhoneOnlyDownload','Assert-ChatGptWebSmokeTrustedDevice',
    'existing_work_in_progress','single_folder_fixture_unavailable','bounded_png_fixture_unavailable',
    'folder_scope_mismatch','native_download_missing','download_receipt_unconfirmed',
    "Probe 'stop'",'Stop-ChatGptWebSmokeAwakeLease','content_exported = $false',
    '$report.passed -and $report.restored -and $report.awake_restored')) {
    if (-not $source.Contains($guard)) { throw "missing_folder_guard:$guard" }
}
foreach ($guard in @('selected_file_mismatch','saved_file_not_unique','saved_file_size_mismatch',
    'saved_png_decode_failed','download_directory_too_large','expected <= 524288',
    'created.removeAll(before)','content_exported','bitmap.recycle()')) {
    if (-not $java.Contains($guard)) { throw "missing_phone_bytes_guard:$guard" }
}
foreach ($forbidden in @('getUiDevice().click(', 'dumpWindowHierarchy', 'file.delete()', 'toByteArray()')) {
    if ($java.Contains($forbidden)) { throw 'unsafe_phone_download_acceptance' }
}
if ($source -match "(?:'pull'|chatgpt_mutate_library_file|chatgpt_send_prompt|chatgpt_attach_library_file)" -or
    $java -match '\.put\("(?:filename|path|content|image|base64)",') { throw 'private_file_export_or_mutation' }
'LIBRARY_FOLDER_DOWNLOAD_CONTRACT=passed'

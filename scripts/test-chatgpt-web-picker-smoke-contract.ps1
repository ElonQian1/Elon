$ErrorActionPreference = "Stop"

$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot "smoke-chatgpt-web-pickers.ps1") -Raw

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) { throw "Picker smoke contract is missing: $Needle" }
}

Assert-Contains '$cameraLabel = ([string][char]0x76F8) + [char]0x673A'
Assert-Contains '$photoLabel = ([string][char]0x7167) + [char]0x7247'
Assert-Contains '$fileLabel = ([string][char]0x6587) + [char]0x4EF6'
Assert-Contains 'chatgpt_select_composer_option'
Assert-Contains 'topResumedActivity='
Assert-Contains 'if ($null -eq $line) { return "" }'
Assert-Contains 'Restore-ChatGptActivity'
Assert-Contains '$pickerPredicate = {'
Assert-Contains '}.GetNewClosure()'
Assert-Contains 'function Wait-ChatGptReady'
Assert-Contains 'Wait-ChatGptReady | Out-Null'
Assert-Contains '$state.composer_ready -eq $true'
Assert-Contains 'selected_local_files = 0'
Assert-Contains 'uploaded_attachments = 0'
Assert-Contains '@($state.conversation.attachments).Count -eq 0'

if ($source -match 'input\s+tap|input\s+text|KEYCODE_ENTER|keyevent\s+66') {
    throw "Picker smoke must not select or confirm a local file."
}

Write-Output "CHATGPT_WEB_PICKER_SMOKE_CONTRACT=passed"

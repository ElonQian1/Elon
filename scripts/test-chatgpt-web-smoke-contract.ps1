$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-apk.ps1"
$source = Get-Content -LiteralPath $sourcePath -Raw

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "ChatGPT Web smoke contract is missing: $Needle"
    }
}

Assert-Contains 'Invoke-UiAction -Action "chatgpt_list_features"'
Assert-Contains 'Get-ComposerOptions -Section "model"'
Assert-Contains 'Get-ComposerOptions -Section "tools"'
Assert-Contains 'Wait-CommandResult -Action $commandAction'
Assert-Contains 'Get-ForeignComposerLabels -Options $modelOptions'
Assert-Contains 'Get-ForeignComposerLabels -Options $toolOptions'
Assert-Contains 'Add-Check "composer_model_scope"'
Assert-Contains 'Add-Check "composer_tool_scope"'
Assert-Contains 'Invoke-Adb shell input keyevent 4'

$featuresIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_list_features"')
$modelIndex = $source.IndexOf('Get-ComposerOptions -Section "model"')
$toolsIndex = $source.IndexOf('Get-ComposerOptions -Section "tools"')
if (-not ($featuresIndex -lt $modelIndex -and $modelIndex -lt $toolsIndex)) {
    throw "Composer contamination smoke must open the sidebar before model and tools checks."
}

Write-Output "CHATGPT_WEB_SMOKE_CONTRACT=passed"

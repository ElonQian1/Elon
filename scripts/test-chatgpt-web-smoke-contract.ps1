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
Assert-Contains 'Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }'
Assert-Contains 'function Wait-NavigationReady'
Assert-Contains 'function Wait-ComposerOptionsReady'
Assert-Contains '$freshCollection = $command.action -eq $expectedAction'
Assert-Contains '$cachedSnapshot = $navigation.control_ok -eq $true -and $options.Count -gt 0'
Assert-Contains 'Wait-ComposerOptionsReady -Section $Section -AfterMs $afterMs'
Assert-Contains '$command.action -eq "collect_navigation"'
Assert-Contains '$command.action -eq "list_navigation"'
Assert-Contains '@($last.features).Count -gt 0'
Assert-Contains 'Wait-NavigationReady -AfterMs $beforeFeatures'
Assert-Contains '$navigationMatrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"'
Assert-Contains 'Add-Check "navigation_adaptation_review"'
Assert-Contains '$navigationCloseCount = [int]$navigationMatrix.observed_semantics.close'
Assert-Contains 'Add-Check "navigation_overlay_open" ($navigationCloseCount -gt 0)'
Assert-Contains '$beforeListState = Invoke-ApkMcp -Tool "ui_state"'
Assert-Contains '$beforeList = [long]$beforeListState.last_command.observed_at_ms'
Assert-Contains 'function Get-TopResumedActivity'
Assert-Contains 'Add-Check "chatgpt_activity_foreground"'
Assert-Contains 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
Assert-Contains 'Get-ComposerOptions -Section "model"'
Assert-Contains 'Get-ComposerOptions -Section "tools"'
Assert-Contains 'Get-ForeignComposerLabels -Options $modelOptions'
Assert-Contains 'Get-ForeignComposerLabels -Options $toolOptions'
Assert-Contains 'Add-Check "composer_model_scope"'
Assert-Contains 'Add-Check "composer_tool_scope"'
Assert-Contains '$adaptationRequired = $matrix.adaptation_review.required -eq $true'
Assert-Contains 'Add-Check "adaptation_review" (-not $adaptationRequired)'
Assert-Contains 'Invoke-Adb shell input keyevent 4'

if ($source.Contains('Wait-CommandResult -Action "collect_navigation" -AfterMs $beforeFeatures')) {
    throw "Navigation smoke must accept the already-collected snapshot path."
}
if ($source.Contains('Wait-CommandResult -Action $commandAction')) {
    throw "Composer smoke must tolerate a newer command overwriting last_command."
}
if ($source.Contains('ToUnixTimeMilliseconds()')) {
    throw "ChatGPT Web smoke must compare bridge timestamps from the same device clock."
}

$featuresIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_list_features"')
$openIndex = $source.IndexOf('Invoke-UiAction -Action "open_chatgpt_web"')
$officialIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }')
$modelIndex = $source.IndexOf('Get-ComposerOptions -Section "model"')
$toolsIndex = $source.IndexOf('Get-ComposerOptions -Section "tools"')
if (-not ($openIndex -lt $officialIndex -and $officialIndex -lt $featuresIndex)) {
    throw "ChatGPT Web smoke must select the official view before readiness and navigation checks."
}
if (-not ($featuresIndex -lt $modelIndex -and $modelIndex -lt $toolsIndex)) {
    throw "Composer contamination smoke must open the sidebar before model and tools checks."
}

Write-Output "CHATGPT_WEB_SMOKE_CONTRACT=passed"

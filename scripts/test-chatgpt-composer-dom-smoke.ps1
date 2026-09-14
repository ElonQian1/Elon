#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-composer-dom-smoke.ps1')

function Get-Command {
    param($Name, $CommandType)
    @([pscustomobject]@{Source='C:/first/node.exe'}, [pscustomobject]@{Source='C:/second/node.exe'})
}
function Invoke-ElonNativeCommand {
    param([string]$FilePath, $ArgumentList, $TimeoutSeconds, $Label)
    if ($FilePath -cne 'C:/first/node.exe') { throw 'node_resolution_not_single_first_path' }
    return @{ Stdout='{"schema":"elon.composer_dom_lease.v1","active":true,"initial_visible":1,"visible_now":0,"samples":2,"visible_samples":0,"invalid_samples":0}' }
}
function Assert-ElonNativeCommand { param($Result,$FailureMessage) }
$state = Invoke-ChatGptComposerDomLease -Lease @{endpoint='http://127.0.0.1:9222';nonce=('a'*32);version=410} -Action state
Assert-ChatGptComposerDomUnavailable $state
$bad = @(
    @{schema='unknown'}, @{active=$false}, @{initial_visible=0}, @{visible_now=1},
    @{samples=0}, @{visible_samples=1}, @{invalid_samples=1}
)
foreach ($change in $bad) {
    $candidate = $state | ConvertTo-Json | ConvertFrom-Json
    foreach($entry in $change.GetEnumerator()) { $candidate.($entry.Key) = $entry.Value }
    $rejected = $false
    try { Assert-ChatGptComposerDomUnavailable $candidate } catch { $rejected = $true }
    if (!$rejected) { throw 'invalid_composer_evidence_accepted' }
}
'COMPOSER_DOM_SMOKE=passed node_path=single evidence_negatives=7'

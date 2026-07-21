$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$script = Join-Path $repoRoot 'scripts\compare-ai-workflow-efficiency.ps1'
$root = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-workflow-metrics-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $baseline = Join-Path $root 'baseline.json'
    $candidate = Join-Path $root 'candidate.json'
    [System.IO.File]::WriteAllText($baseline, '{"taskFingerprint":"task-1","acceptanceCriteriaDigest":"criteria-1","modelProfile":"gpt-5.6-medium","inputTokens":1000,"cachedInputTokens":400,"outputTokens":200,"durationMs":10000,"eventCount":50,"failureCount":2,"failedTools":2,"waitPayloadBytes":50000}')
    [System.IO.File]::WriteAllText($candidate, '{"taskFingerprint":"task-1","acceptanceCriteriaDigest":"criteria-1","modelProfile":"gpt-5.6-medium","inputTokens":700,"cachedInputTokens":400,"outputTokens":150,"durationMs":7000,"eventCount":50,"failureCount":0,"failedTools":0,"waitPayloadBytes":10000}')
    $result = & powershell -NoProfile -ExecutionPolicy Bypass -File $script -BaselinePath $baseline -CandidatePath $candidate | ConvertFrom-Json
    if ($result.schema -ne 'elon.ai_workflow_efficiency_comparison.v1' -or
        $result.delta.uncachedInputTokens -ne -300 -or
        $result.delta.waitPayloadBytesPercent -ne -80 -or
        $result.evidencePolicy -ne 'MATCHED_TASKS_ONLY') {
        throw 'Workflow efficiency comparison returned unexpected metrics.'
    }
    [System.IO.File]::WriteAllText($candidate, '{"taskFingerprint":"different-task","acceptanceCriteriaDigest":"criteria-1","modelProfile":"gpt-5.6-medium","inputTokens":700,"cachedInputTokens":400,"outputTokens":150,"durationMs":7000,"eventCount":50,"failureCount":0,"failedTools":0,"waitPayloadBytes":10000}')
    $mismatchRejected = $false
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $script -BaselinePath $baseline -CandidatePath $candidate *> $null
        if ($LASTEXITCODE -ne 0) { $mismatchRejected = $true }
    } catch {
        $mismatchRejected = $true
    }
    if (-not $mismatchRejected) { throw 'Workflow efficiency comparison accepted mismatched tasks.' }
    Write-Host 'WORKFLOW_EFFICIENCY_TEST=passed'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

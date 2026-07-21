param(
    [Parameter(Mandatory = $true)][string]$BaselinePath,
    [Parameter(Mandatory = $true)][string]$CandidatePath
)

$ErrorActionPreference = 'Stop'

function Read-Metrics {
    param([string]$Path, [string]$Label)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "$Label metrics file not found: $Path" }
    $value = Get-Content -Raw -LiteralPath $Path -Encoding UTF8 | ConvertFrom-Json
    $identityFields = @('taskFingerprint','acceptanceCriteriaDigest','modelProfile')
    foreach ($field in $identityFields) {
        if ($null -eq $value.PSObject.Properties[$field] -or [string]::IsNullOrWhiteSpace([string]$value.$field)) {
            throw "$Label metrics identity field is missing: $field"
        }
    }
    $required = @('inputTokens','cachedInputTokens','outputTokens','durationMs','eventCount','failureCount','failedTools','waitPayloadBytes')
    foreach ($field in $required) {
        if ($null -eq $value.PSObject.Properties[$field] -or [double]$value.$field -lt 0) {
            throw "$Label metrics field is missing or negative: $field"
        }
    }
    $inputTokens = [double]$value.inputTokens
    $cachedTokens = [double]$value.cachedInputTokens
    if ($cachedTokens -gt $inputTokens) { throw "$Label cachedInputTokens exceeds inputTokens" }
    [ordered]@{
        taskFingerprint = [string]$value.taskFingerprint
        acceptanceCriteriaDigest = [string]$value.acceptanceCriteriaDigest
        modelProfile = [string]$value.modelProfile
        inputTokens = $inputTokens
        cachedInputTokens = $cachedTokens
        uncachedInputTokens = $inputTokens - $cachedTokens
        outputTokens = [double]$value.outputTokens
        cacheRate = if ($inputTokens -eq 0) { 0 } else { [Math]::Round($cachedTokens / $inputTokens, 6) }
        durationMs = [double]$value.durationMs
        eventCount = [double]$value.eventCount
        failureCount = [double]$value.failureCount
        failedTools = [double]$value.failedTools
        waitPayloadBytes = [double]$value.waitPayloadBytes
    }
}

function Percent-Change {
    param([double]$Before, [double]$After)
    if ($Before -eq 0) { return $null }
    [Math]::Round((($After - $Before) / $Before) * 100, 2)
}

$baseline = Read-Metrics $BaselinePath 'baseline'
$candidate = Read-Metrics $CandidatePath 'candidate'
foreach ($identityField in @('taskFingerprint','acceptanceCriteriaDigest','modelProfile')) {
    if ($baseline.$identityField -cne $candidate.$identityField) {
        throw "A/B comparison rejected: $identityField does not match."
    }
}
$comparison = [ordered]@{
    schema = 'elon.ai_workflow_efficiency_comparison.v1'
    evidencePolicy = 'MATCHED_TASKS_ONLY'
    baseline = $baseline
    candidate = $candidate
    delta = [ordered]@{
        inputTokens = $candidate.inputTokens - $baseline.inputTokens
        cachedInputTokens = $candidate.cachedInputTokens - $baseline.cachedInputTokens
        uncachedInputTokens = $candidate.uncachedInputTokens - $baseline.uncachedInputTokens
        uncachedInputTokensPercent = Percent-Change $baseline.uncachedInputTokens $candidate.uncachedInputTokens
        outputTokens = $candidate.outputTokens - $baseline.outputTokens
        durationMs = $candidate.durationMs - $baseline.durationMs
        durationPercent = Percent-Change $baseline.durationMs $candidate.durationMs
        eventCount = $candidate.eventCount - $baseline.eventCount
        failureCount = $candidate.failureCount - $baseline.failureCount
        failedTools = $candidate.failedTools - $baseline.failedTools
        waitPayloadBytes = $candidate.waitPayloadBytes - $baseline.waitPayloadBytes
        waitPayloadBytesPercent = Percent-Change $baseline.waitPayloadBytes $candidate.waitPayloadBytes
    }
    interpretation = 'Negative deltas mean savings. Do not claim savings unless baseline and candidate are matched executions of the same acceptance criteria.'
}
$comparison | ConvertTo-Json -Depth 8

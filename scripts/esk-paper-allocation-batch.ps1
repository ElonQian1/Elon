[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,

    [string]$BaseUrl = "http://127.0.0.1:8080",

    [ValidatePattern('^[A-Za-z_][A-Za-z0-9_]*$')]
    [string]$AdminTokenEnvironment = "ELON_ADMIN_TOKEN",

    [switch]$Commit,

    [ValidatePattern('^[0-9a-f]{64}$')]
    [string]$ExpectedRequestDigest,

    [string]$ReceiptPath
)

$ErrorActionPreference = "Stop"

function Invoke-EskBatchApi {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Uri,
        [Parameter(Mandatory = $true)]
        [hashtable]$Headers,
        [Parameter(Mandatory = $true)]
        [hashtable]$Body
    )

    try {
        $json = $Body | ConvertTo-Json -Depth 8 -Compress
        return Invoke-RestMethod -Method Post -Uri $Uri -Headers $Headers `
            -ContentType "application/json" -Body $json -TimeoutSec 30
    } catch {
        $status = $null
        if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
            $status = [int]$_.Exception.Response.StatusCode
        }
        if ($status) {
            throw "ESK Paper batch API request failed with HTTP $status."
        }
        throw "ESK Paper batch API request failed before a response was received."
    }
}

function Assert-InputShape {
    param([Parameter(Mandatory = $true)]$InputObject)

    $topLevelNames = @($InputObject.PSObject.Properties.Name)
    $unexpected = @($topLevelNames | Where-Object { $_ -notin @("batch_id", "entries") })
    if ($unexpected.Count -gt 0) {
        throw "Input JSON contains unsupported top-level fields. Use only batch_id and entries."
    }
    if ([string]::IsNullOrWhiteSpace([string]$InputObject.batch_id)) {
        throw "Input JSON batch_id is required."
    }
    if ($null -eq $InputObject.entries) {
        throw "Input JSON entries are required."
    }
    $entries = @($InputObject.entries)
    if ($entries.Count -lt 1 -or $entries.Count -gt 100) {
        throw "Input JSON entries must contain 1..100 items."
    }
    foreach ($entry in $entries) {
        if ($null -eq $entry) {
            throw "Input JSON entries cannot contain null items."
        }
        $names = @($entry.PSObject.Properties.Name)
        $extra = @($names | Where-Object {
            $_ -notin @("user_id", "amount", "reference", "idempotency_key")
        })
        if ($extra.Count -gt 0) {
            throw "An input entry contains unsupported fields. Do not include personal or payment data."
        }
        foreach ($required in @("user_id", "amount", "reference", "idempotency_key")) {
            if ([string]::IsNullOrWhiteSpace([string]$entry.$required)) {
                throw "Every input entry requires user_id, amount, reference and idempotency_key."
            }
        }
    }
}

function Write-ReceiptIfRequested {
    param(
        [Parameter(Mandatory = $true)]$Receipt,
        [string]$Path
    )

    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $parent = Split-Path -Parent $fullPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    }
    $json = $Receipt | ConvertTo-Json -Depth 10
    [System.IO.File]::WriteAllText($fullPath, $json, [System.Text.UTF8Encoding]::new($false))
    return $fullPath
}

$resolvedInput = (Resolve-Path -LiteralPath $InputPath -ErrorAction Stop).Path
$inputObject = Get-Content -Raw -LiteralPath $resolvedInput -Encoding UTF8 | ConvertFrom-Json
Assert-InputShape -InputObject $inputObject

$adminToken = [Environment]::GetEnvironmentVariable($AdminTokenEnvironment, "Process")
if ([string]::IsNullOrWhiteSpace($adminToken)) {
    throw "Administrator token environment variable is missing or empty."
}

$endpoint = $BaseUrl.TrimEnd('/') + "/api/admin/assets/esk/paper-allocation-batches"
$headers = @{ Authorization = "Bearer $adminToken" }
$entries = @($inputObject.entries)
$previewBody = @{
    batch_id = [string]$inputObject.batch_id
    mode = "dry_run"
    entries = $entries
}
$preview = Invoke-EskBatchApi -Uri $endpoint -Headers $headers -Body $previewBody
if ($preview.schema -ne "yilong.esk.paper_allocation_batch_receipt.v1" -or
    $preview.status -ne "validated" -or
    [string]::IsNullOrWhiteSpace([string]$preview.request_digest)) {
    throw "ESK Paper batch dry-run returned an invalid receipt."
}

$receipt = $preview
if ($Commit) {
    if ([string]::IsNullOrWhiteSpace($ExpectedRequestDigest)) {
        throw "Commit requires -ExpectedRequestDigest from a prior dry-run."
    }
    if ($ExpectedRequestDigest -cne [string]$preview.request_digest) {
        throw "Input changed after dry-run: expected request digest does not match."
    }
    $commitBody = @{
        batch_id = [string]$inputObject.batch_id
        mode = "commit"
        expected_request_digest = $ExpectedRequestDigest
        confirmation = "RECORD PAPER ESK BATCH"
        entries = $entries
    }
    $receipt = Invoke-EskBatchApi -Uri $endpoint -Headers $headers -Body $commitBody
    if ($receipt.schema -ne "yilong.esk.paper_allocation_batch_receipt.v1" -or
        $receipt.status -ne "committed" -or
        [string]$receipt.request_digest -cne $ExpectedRequestDigest) {
        throw "ESK Paper batch commit returned an invalid receipt."
    }
}

$writtenReceipt = Write-ReceiptIfRequested -Receipt $receipt -Path $ReceiptPath
[ordered]@{
    schema = "yilong.esk.paper_allocation_batch_operator_summary.v1"
    batch_id = [string]$receipt.batch_id
    operation = if ($Commit) { "commit" } else { "dry_run" }
    status = [string]$receipt.status
    request_digest = [string]$receipt.request_digest
    entry_count = [int]$receipt.entry_count
    total = [string]$receipt.total
    replayed = [bool]$receipt.replayed
    created_at = $receipt.created_at
    receipt_path = $writtenReceipt
    simulated = $true
    funds_moved = $false
} | ConvertTo-Json -Depth 4

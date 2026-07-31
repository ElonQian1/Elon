param(
    [string]$CoffeeRepo = "D:\rust\active-projects\cofficethinking"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$platformContract = Join-Path $repoRoot "contracts\open-commerce\merchant-runtime-v1.json"
$coffeeContract = Join-Path $CoffeeRepo "contracts\open-commerce\merchant-runtime-v1.json"

function Get-NormalizedTextHash {
    param([Parameter(Mandatory = $true)][string]$Path)

    $content = [System.IO.File]::ReadAllText($Path)
    $normalized = $content.Replace("`r`n", "`n").Replace("`r", "`n")
    $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($normalized)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return -join ($sha256.ComputeHash($bytes) | ForEach-Object { $_.ToString("X2") })
    } finally {
        $sha256.Dispose()
    }
}

if (-not (Test-Path -LiteralPath $platformContract -PathType Leaf)) {
    throw "Platform merchant runtime contract is missing: $platformContract"
}
if (-not (Test-Path -LiteralPath $coffeeContract -PathType Leaf)) {
    throw "Coffee merchant runtime contract is missing: $coffeeContract"
}

$platformHash = Get-NormalizedTextHash -Path $platformContract
$coffeeHash = Get-NormalizedTextHash -Path $coffeeContract
if ($platformHash -ne $coffeeHash) {
    throw "Merchant runtime contracts drifted: platform=$platformHash coffee=$coffeeHash"
}

$platformImplementation = @(
    Get-Content -Raw -LiteralPath (Join-Path $repoRoot "server\src\open_commerce_runtime_client.rs")
    Get-Content -Raw -LiteralPath (Join-Path $repoRoot "server\src\open_commerce_service.rs")
) -join "`n"
$coffeeSecurity = Get-Content -Raw -LiteralPath (Join-Path $CoffeeRepo "services\backend\src\modules\commerce_gateway\security.rs")
$coffeeService = Get-Content -Raw -LiteralPath (Join-Path $CoffeeRepo "services\backend\src\modules\commerce_gateway\service.rs")

foreach ($required in @(
    "merchant_runtime.invoke.v1",
    "merchant_runtime.result.v1",
    "x-yilong-runtime-signature"
)) {
    if (-not $platformImplementation.Contains($required)) {
        throw "Platform runtime implementation does not implement contract token: $required"
    }
}
foreach ($required in @(
    "x-yilong-runtime-key-id",
    "x-yilong-runtime-timestamp",
    "x-yilong-runtime-signature"
)) {
    if (-not $coffeeSecurity.Contains($required)) {
        throw "Coffee runtime security does not implement contract header: $required"
    }
}
foreach ($required in @(
    '"order.quote.create"',
    '"order.commit"',
    '"order.status.read"',
    "confirmed_by_user"
)) {
    if (-not $coffeeService.Contains($required)) {
        throw "Coffee runtime service does not implement contract guard: $required"
    }
}

Write-Output "OPEN_COMMERCE_MERCHANT_RUNTIME_CONTRACT=passed"
Write-Output "CONTRACT_SHA256=$platformHash"

[CmdletBinding()]
param(
    [string]$CoffeeRepo = "D:\rust\active-projects\cofficethinking",
    [string]$CoffeeServerConfig = "",
    [string]$RuntimeEndpoint = "https://182.254.168.75",
    [switch]$AcknowledgeProductionWrite
)

$ErrorActionPreference = "Stop"
$Acknowledgement = "I_ACCEPT_ONE_UNPAID_ORDER_IN_THE_SUSPENDED_ACCEPTANCE_STORE"
$AcceptanceStoreId = "a11c0000-0000-4000-8000-000000000001"
$AcceptanceMerchantId = "merchant-cofficethinking-acceptance"
$RuntimeSecretRef = "OPEN_COMMERCE_RUNTIME_SECRET_COFFICE"
$ExpectedEndpoint = "https://182.254.168.75"

if (-not $AcknowledgeProductionWrite) {
    throw "This acceptance creates one unpaid order in the dedicated suspended store. Re-run with -AcknowledgeProductionWrite."
}
if ($RuntimeEndpoint.TrimEnd('/') -ne $ExpectedEndpoint) {
    throw "RuntimeEndpoint must remain pinned to the approved acceptance endpoint: $ExpectedEndpoint"
}
foreach ($command in @("git", "ssh", "powershell")) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $command"
    }
}

function Read-KeyValueFile {
    param([Parameter(Mandatory = $true)][string]$Path)

    $values = @{}
    Get-Content -LiteralPath $Path | ForEach-Object {
        $line = $_.Trim()
        if ($line -and -not $line.StartsWith("#")) {
            $parts = $line.Split("=", 2)
            if ($parts.Count -eq 2 -and $parts[0].Trim()) {
                $values[$parts[0].Trim()] = $parts[1].Trim()
            }
        }
    }
    return $values
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$coffeeRoot = [System.IO.Path]::GetFullPath($CoffeeRepo)
$configPath = if ($CoffeeServerConfig) {
    [System.IO.Path]::GetFullPath($CoffeeServerConfig)
} else {
    Join-Path $coffeeRoot "deploy\server.env"
}
if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
    throw "Coffee server config not found: $configPath"
}
$config = Read-KeyValueFile -Path $configPath
$serverHost = $config["SERVER_HOST"]
$serverUser = $config["SERVER_USER"]
$serverPort = if ($config["SERVER_PORT"]) { [int]$config["SERVER_PORT"] } else { 22 }
$appDir = if ($config["REMOTE_APP_DIR"]) { $config["REMOTE_APP_DIR"] } else { "/opt/cofficethinking/backend" }
if (-not $serverHost -or -not $serverUser) {
    throw "SERVER_HOST and SERVER_USER are required in $configPath"
}

$runId = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds().ToString()
$sku = "YILONG-PUBLIC-HTTPS-$runId"
$sshTarget = "$serverUser@$serverHost"
$sshArgs = @(
    "-p", "$serverPort",
    "-o", "ProxyCommand=none",
    "-o", "ProxyJump=none",
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=10",
    $sshTarget
)
$prepareScript = @'
set -euo pipefail
cd "$APP_DIR"
. ./.env
test "$OPEN_COMMERCE_RUNTIME_ENABLED" = "true"
test "$OPEN_COMMERCE_STORE_ID" = "$EXPECTED_STORE_ID"
test "$OPEN_COMMERCE_MERCHANT_ID" = "$EXPECTED_MERCHANT_ID"
test "$OPEN_COMMERCE_RUNTIME_KEY_ID" = "$EXPECTED_KEY_ID"
test "${#OPEN_COMMERCE_RUNTIME_SHARED_SECRET}" -ge 32
store_status="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT status FROM stores WHERE id='$EXPECTED_STORE_ID'::uuid")"
test "$store_status" = "suspended"
offer="$(jq -cn --arg sku "$SKU" '{sku:$sku,name:"Yilong public HTTPS acceptance latte",description:"Dedicated suspended-store acceptance item",unit_price_minor:2600,currency:"CNY",stock_quantity:5,is_active:true,attributes:{acceptance_only:true,public_https:true}}' | curl -fsS -X POST http://127.0.0.1:8080/api/v1/open-commerce/offers -H 'content-type: application/json' -H "x-commerce-admin-token: $OPEN_COMMERCE_ADMIN_TOKEN" --data-binary @-)"
printf 'OFFER_ID=%s\n' "$(printf '%s' "$offer" | jq -er '.offer.id')"
printf 'START_STOCK=%s\n' "$(printf '%s' "$offer" | jq -er '.offer.stock_quantity')"
printf 'RUNTIME_KEY_ID=%s\n' "$OPEN_COMMERCE_RUNTIME_KEY_ID"
printf 'RUNTIME_SECRET_B64=%s\n' "$(printf '%s' "$OPEN_COMMERCE_RUNTIME_SHARED_SECRET" | base64 -w0)"
'@
$prepareBytes = [Text.Encoding]::UTF8.GetBytes($prepareScript)
$prepareEncoded = [Convert]::ToBase64String($prepareBytes)
$remotePrepare = "echo '$prepareEncoded' | base64 -d | APP_DIR='$appDir' EXPECTED_STORE_ID='$AcceptanceStoreId' EXPECTED_MERCHANT_ID='$AcceptanceMerchantId' EXPECTED_KEY_ID='$RuntimeSecretRef' SKU='$sku' bash"

Write-Host "[public-https] preparing one isolated offer in the suspended acceptance store"
$prepareOutput = @(& ssh @sshArgs $remotePrepare)
if ($LASTEXITCODE -ne 0) {
    throw "Coffee acceptance offer preparation failed with exit code $LASTEXITCODE"
}
$prepared = @{}
foreach ($line in $prepareOutput) {
    $parts = ([string]$line).Split("=", 2)
    if ($parts.Count -eq 2) { $prepared[$parts[0]] = $parts[1] }
}
$offerId = $prepared["OFFER_ID"]
$startStock = $prepared["START_STOCK"]
$runtimeKeyId = $prepared["RUNTIME_KEY_ID"]
$secretB64 = $prepared["RUNTIME_SECRET_B64"]
if (-not $offerId -or -not $secretB64 -or $runtimeKeyId -ne $RuntimeSecretRef -or $startStock -ne "5") {
    throw "Coffee acceptance preparation returned an invalid or unsafe identity"
}
$runtimeSecret = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($secretB64))
if ($runtimeSecret.Length -lt 32) { throw "Runtime secret is too short" }

$receiptDir = Join-Path $repoRoot ".ai-tmp"
New-Item -ItemType Directory -Path $receiptDir -Force | Out-Null
$receiptPath = Join-Path $receiptDir "open-commerce-public-https-$runId.json"
$previous = @{}
foreach ($name in @(
    "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ACCEPTANCE",
    "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT",
    "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_OFFER_ID",
    "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RUN_ID",
    "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RECEIPT_PATH",
    $RuntimeSecretRef,
    "OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS"
)) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
try {
    $env:ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ACCEPTANCE = $Acknowledgement
    $env:ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT = $RuntimeEndpoint.TrimEnd('/')
    $env:ELON_OPEN_COMMERCE_PUBLIC_HTTPS_OFFER_ID = $offerId
    $env:ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RUN_ID = $runId
    $env:ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RECEIPT_PATH = $receiptPath
    [Environment]::SetEnvironmentVariable($RuntimeSecretRef, $runtimeSecret, "Process")
    $env:OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS = ([Uri]$RuntimeEndpoint).Host

    Write-Host "[public-https] running the real platform credential, confirmation, invocation and closure path"
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\validate-rust.ps1") `
        -Force -Domain open-commerce-public-https-acceptance -- `
        test --manifest-path (Join-Path $repoRoot "server\Cargo.toml") `
        open_commerce_runtime_service_tests::public_https_acceptance_tests::real_consumer_ai_order_reaches_public_coffee_erp `
        -- --exact --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Platform public HTTPS acceptance test failed with exit code $LASTEXITCODE"
    }
} finally {
    foreach ($entry in $previous.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, "Process")
    }
    $runtimeSecret = $null
    $secretB64 = $null
}
if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) {
    throw "Platform acceptance receipt was not produced"
}
$receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
if ($receipt.schema -ne "open_commerce.public_https_acceptance.v1" -or
    $receipt.payment_status -ne "unpaid" -or
    $receipt.funds_moved -ne $false -or
    $receipt.platform_store -ne "isolated_temporary_sqlite") {
    throw "Platform acceptance receipt violated the zero-funds or isolation boundary"
}

Write-Host "[public-https] cross-checking the same order in PostgreSQL and the ERP API"
$verifyArgs = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", (Join-Path $repoRoot "scripts\verify-open-commerce-public-https-coffee.ps1"),
    "-CoffeeRepo", $coffeeRoot,
    "-ReceiptPath", $receiptPath,
    "-ExpectedOfferId", $offerId,
    "-ExpectedStartStock", $startStock
)
if ($CoffeeServerConfig) {
    $verifyArgs += @("-CoffeeServerConfig", $configPath)
}
$verificationJson = (& powershell @verifyArgs | Select-Object -Last 1)
if ($LASTEXITCODE -ne 0) {
    throw "Coffee ERP cross-check failed with exit code $LASTEXITCODE"
}
$verification = $verificationJson | ConvertFrom-Json
if ($verification.schema -ne "open_commerce.public_https_erp_verification.v1" -or
    $verification.unified_order_id -ne $receipt.unified_order_id -or
    $verification.funds_moved -ne $false) {
    throw "Coffee ERP verification identity did not match the platform receipt"
}

[pscustomobject]@{
    schema = "open_commerce.public_https_full_acceptance.v1"
    runtime_endpoint = $RuntimeEndpoint.TrimEnd('/')
    merchant_id = $AcceptanceMerchantId
    invocation_id = $receipt.invocation_id
    order_id = $receipt.order_id
    unified_order_id = $receipt.unified_order_id
    payment_status = "unpaid"
    funds_moved = $false
    idempotent_replay = $true
    platform_store = "isolated_temporary_sqlite"
    merchant_database = "real_postgresql"
    erp_api_match_count = $verification.erp_api_match_count
    commit_receipt_count = $verification.commit_receipt_count
    start_stock = $verification.start_stock
    end_stock = $verification.end_stock
} | ConvertTo-Json -Compress

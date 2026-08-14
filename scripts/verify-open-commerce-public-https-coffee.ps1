[CmdletBinding()]
param(
    [string]$CoffeeRepo = "D:\rust\active-projects\cofficethinking",
    [string]$CoffeeServerConfig = "",
    [Parameter(Mandatory = $true)][string]$ReceiptPath,
    [string]$ExpectedOfferId = "",
    [int]$ExpectedStartStock = 5
)

$ErrorActionPreference = "Stop"
$AcceptanceStoreId = "a11c0000-0000-4000-8000-000000000001"
$AcceptanceMerchantId = "merchant-cofficethinking-acceptance"
$ExpectedEndpoint = "https://182.254.168.75"

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

$coffeeRoot = [System.IO.Path]::GetFullPath($CoffeeRepo)
$configPath = if ($CoffeeServerConfig) {
    [System.IO.Path]::GetFullPath($CoffeeServerConfig)
} else {
    Join-Path $coffeeRoot "deploy\server.env"
}
$receiptFullPath = [System.IO.Path]::GetFullPath($ReceiptPath)
if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
    throw "Coffee server config not found: $configPath"
}
if (-not (Test-Path -LiteralPath $receiptFullPath -PathType Leaf)) {
    throw "Acceptance receipt not found: $receiptFullPath"
}
if ($ExpectedStartStock -lt 1) {
    throw "ExpectedStartStock must be positive"
}

$receipt = Get-Content -Raw -LiteralPath $receiptFullPath | ConvertFrom-Json
if ($receipt.schema -ne "open_commerce.public_https_acceptance.v1" -or
    $receipt.endpoint -ne $ExpectedEndpoint -or
    $receipt.merchant_id -ne $AcceptanceMerchantId -or
    $receipt.payment_status -ne "unpaid" -or
    $receipt.funds_moved -ne $false -or
    $receipt.platform_store -ne "isolated_temporary_sqlite") {
    throw "Acceptance receipt violated the expected identity or zero-funds boundary"
}

$config = Read-KeyValueFile -Path $configPath
$serverHost = $config["SERVER_HOST"]
$serverUser = $config["SERVER_USER"]
$serverPort = if ($config["SERVER_PORT"]) { [int]$config["SERVER_PORT"] } else { 22 }
$appDir = if ($config["REMOTE_APP_DIR"]) { $config["REMOTE_APP_DIR"] } else { "/opt/cofficethinking/backend" }
if (-not $serverHost -or -not $serverUser) {
    throw "SERVER_HOST and SERVER_USER are required in $configPath"
}
$sshArgs = @(
    "-p", "$serverPort",
    "-o", "ProxyCommand=none",
    "-o", "ProxyJump=none",
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=10",
    "$serverUser@$serverHost"
)

$verifyScript = @'
set -euo pipefail
cd "$APP_DIR"
. ./.env
test "$OPEN_COMMERCE_STORE_ID" = "$EXPECTED_STORE_ID"
test "$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT status FROM stores WHERE id='$EXPECTED_STORE_ID'::uuid")" = "suspended"
offer_id="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT q.items_json->0->>'product_id' FROM open_commerce_orders oc JOIN open_commerce_quotes q ON q.id=oc.quote_id WHERE oc.id='$ORDER_ID'::uuid AND oc.unified_order_id='$UNIFIED_ORDER_ID'::uuid AND oc.merchant_id='$EXPECTED_MERCHANT_ID'")"
test -n "$offer_id"
if [ -n "$EXPECTED_OFFER_ID" ]; then test "$offer_id" = "$EXPECTED_OFFER_ID"; fi
offer_sku="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT sku FROM open_commerce_offers WHERE id='$offer_id'::uuid AND store_id='$EXPECTED_STORE_ID'::uuid")"
test "$offer_sku" = "YILONG-PUBLIC-HTTPS-$RUN_ID"
end_stock="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT stock_quantity FROM open_commerce_offers WHERE id='$offer_id'::uuid")"
open_count="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT count(*) FROM open_commerce_orders oc JOIN unified_orders u ON u.id=oc.unified_order_id WHERE oc.id='$ORDER_ID'::uuid AND oc.unified_order_id='$UNIFIED_ORDER_ID'::uuid AND oc.merchant_id='$EXPECTED_MERCHANT_ID' AND u.store_id='$EXPECTED_STORE_ID'::uuid AND u.status='confirmed' AND u.paid_amount=0")"
receipt_count="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -At -c "SELECT count(*) FROM open_commerce_invocation_receipts WHERE merchant_id='$EXPECTED_MERCHANT_ID' AND capability_key='order.commit' AND idempotency_key='$COMMIT_KEY' AND status='succeeded'")"
api_count="$(curl -fsS "http://127.0.0.1:8080/api/v1/orders?store_id=$EXPECTED_STORE_ID" | jq --arg id "$UNIFIED_ORDER_ID" '[.. | objects | select(.id? == $id)] | length')"
expected_end_stock=$((EXPECTED_START_STOCK - 1))
test "$end_stock" = "$expected_end_stock"
test "$open_count" = "1"
test "$receipt_count" = "1"
test "$api_count" = "1"
jq -cn --arg schema open_commerce.public_https_erp_verification.v1 --arg offer_id "$offer_id" --arg order_id "$ORDER_ID" --arg unified_order_id "$UNIFIED_ORDER_ID" --argjson start_stock "$EXPECTED_START_STOCK" --argjson end_stock "$end_stock" --argjson erp_api_match_count "$api_count" --argjson commit_receipt_count "$receipt_count" '{schema:$schema,offer_id:$offer_id,order_id:$order_id,unified_order_id:$unified_order_id,payment_status:"unpaid",funds_moved:false,start_stock:$start_stock,end_stock:$end_stock,erp_api_match_count:$erp_api_match_count,commit_receipt_count:$commit_receipt_count}'
'@
$verifyEncoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($verifyScript))
$remoteVerify = "echo '$verifyEncoded' | base64 -d | APP_DIR='$appDir' EXPECTED_STORE_ID='$AcceptanceStoreId' EXPECTED_MERCHANT_ID='$AcceptanceMerchantId' EXPECTED_OFFER_ID='$ExpectedOfferId' ORDER_ID='$($receipt.order_id)' UNIFIED_ORDER_ID='$($receipt.unified_order_id)' COMMIT_KEY='$($receipt.commit_idempotency_key)' RUN_ID='$($receipt.run_id)' EXPECTED_START_STOCK='$ExpectedStartStock' bash"
$verificationJson = (& ssh @sshArgs $remoteVerify | Select-Object -Last 1)
if ($LASTEXITCODE -ne 0) {
    throw "Coffee ERP read-only verification failed with exit code $LASTEXITCODE"
}
$verification = $verificationJson | ConvertFrom-Json
if ($verification.schema -ne "open_commerce.public_https_erp_verification.v1" -or
    $verification.order_id -ne $receipt.order_id -or
    $verification.unified_order_id -ne $receipt.unified_order_id -or
    $verification.payment_status -ne "unpaid" -or
    $verification.funds_moved -ne $false) {
    throw "Coffee ERP verification did not match the platform acceptance receipt"
}
$verification | ConvertTo-Json -Compress

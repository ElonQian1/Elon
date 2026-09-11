param([switch]$AdapterOnly)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
foreach ($name in @('test-binance-grid-create-adapter.cjs','test-binance-grid-manage-adapter.cjs','test-binance-grid-protection-contract.cjs','test-binance-grid-trailing-rules.cjs','test-binance-grid-trailing-contract.cjs','test-binance-grid-trailing-adapter.cjs','test-binance-grid-read-session.cjs','test-binance-grid-read-diagnostics.cjs','test-binance-grid-read-adapter.cjs')) {
    & node (Join-Path $PSScriptRoot $name)
    if ($LASTEXITCODE -ne 0) { throw "Binance adapter contract failed: $name" }
}
if ($AdapterOnly) {
    Write-Output 'BINANCE_ADAPTER_SUITES_PASSED=9 NATIVE_TESTS_EXECUTED=false TRADING_REQUESTS_EXECUTED=false'
    return
}
$previous = $env:GRADLE_EXIT_CONSOLE
try {
    $env:GRADLE_EXIT_CONSOLE = 'true'
    & (Join-Path $root 'android/gradlew.bat') -p (Join-Path $root 'android') --no-daemon --max-workers=1 :app:testDebugUnitTest `
        --tests com.elon.app.grid.create.BinanceGridCreateTest `
        --tests com.elon.app.grid.manage.BinanceManageStateTest `
        --tests com.elon.app.grid.manage.BinanceManageReadTest `
        --tests com.elon.app.grid.manage.BinanceManageReconnectTest `
        --tests com.elon.app.grid.manage.BinanceInvestmentTest `
        --tests com.elon.app.grid.manage.BinanceRangeTest `
        --tests com.elon.app.grid.manage.BinanceProtectionTest `
        --tests com.elon.app.grid.manage.BinanceTrailingTest `
        --tests com.elon.app.grid.manage.BinanceTrailingMarketTest `
        --tests com.elon.app.grid.manage.BinanceTrailingStateTest `
        --tests com.elon.app.grid.manage.BinanceManageCommandDraftTest `
        --tests com.elon.app.grid.host.BinanceHostStateTest `
        --tests com.elon.app.grid.host.BinanceHostDiagnosticsTest
    if ($LASTEXITCODE -ne 0) { throw 'Binance native compilation or tests failed' }
} finally { $env:GRADLE_EXIT_CONSOLE = $previous }
$count = 0
foreach ($suite in @('com.elon.app.grid.create.BinanceGridCreateTest','com.elon.app.grid.manage.BinanceManageStateTest','com.elon.app.grid.manage.BinanceManageReadTest','com.elon.app.grid.manage.BinanceManageReconnectTest','com.elon.app.grid.manage.BinanceInvestmentTest','com.elon.app.grid.manage.BinanceRangeTest','com.elon.app.grid.manage.BinanceProtectionTest','com.elon.app.grid.manage.BinanceTrailingTest','com.elon.app.grid.manage.BinanceTrailingMarketTest','com.elon.app.grid.manage.BinanceTrailingStateTest','com.elon.app.grid.manage.BinanceManageCommandDraftTest','com.elon.app.grid.host.BinanceHostStateTest','com.elon.app.grid.host.BinanceHostDiagnosticsTest')) {
    $path = Join-Path $root "android/app/build/test-results/testDebugUnitTest/TEST-$suite.xml"
    if (!(Test-Path -LiteralPath $path)) { throw 'Binance JUnit result missing' }
    [xml]$xml = Get-Content -LiteralPath $path -Raw
    if ([int]$xml.testsuite.tests -lt 1 -or [int]$xml.testsuite.failures -ne 0 -or [int]$xml.testsuite.errors -ne 0) { throw "Binance JUnit failed: $suite" }
    $count += [int]$xml.testsuite.tests
}
Write-Output "BINANCE_NATIVE_TESTS_PASSED=$count TRADING_REQUESTS_EXECUTED=false"

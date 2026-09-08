param()
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
foreach ($name in @('test-binance-grid-create-adapter.cjs','test-binance-grid-read-session.cjs','test-binance-grid-read-diagnostics.cjs','test-binance-grid-read-adapter.cjs')) {
    & node (Join-Path $PSScriptRoot $name)
    if ($LASTEXITCODE -ne 0) { throw "Binance adapter contract failed: $name" }
}
$previous = $env:GRADLE_EXIT_CONSOLE
try {
    $env:GRADLE_EXIT_CONSOLE = 'true'
    & (Join-Path $root 'android/gradlew.bat') -p (Join-Path $root 'android') --no-daemon --max-workers=1 :app:testDebugUnitTest `
        --tests com.elon.app.grid.create.BinanceGridCreateTest `
        --tests com.elon.app.grid.host.BinanceHostStateTest `
        --tests com.elon.app.grid.host.BinanceHostDiagnosticsTest
    if ($LASTEXITCODE -ne 0) { throw 'Binance native compilation or tests failed' }
} finally { $env:GRADLE_EXIT_CONSOLE = $previous }
$count = 0
foreach ($suite in @('com.elon.app.grid.create.BinanceGridCreateTest','com.elon.app.grid.host.BinanceHostStateTest','com.elon.app.grid.host.BinanceHostDiagnosticsTest')) {
    $path = Join-Path $root "android/app/build/test-results/testDebugUnitTest/TEST-$suite.xml"
    if (!(Test-Path -LiteralPath $path)) { throw 'Binance JUnit result missing' }
    [xml]$xml = Get-Content -LiteralPath $path -Raw
    if ([int]$xml.testsuite.tests -lt 1 -or [int]$xml.testsuite.failures -ne 0 -or [int]$xml.testsuite.errors -ne 0) { throw "Binance JUnit failed: $suite" }
    $count += [int]$xml.testsuite.tests
}
Write-Output "BINANCE_NATIVE_TESTS_PASSED=$count TRADING_REQUESTS_EXECUTED=false"

$ErrorActionPreference = 'Stop'
$shareRepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location (Join-Path $shareRepoRoot 'android')
try {
    & ./gradlew.bat :app:testDebugUnitTest --tests 'com.elon.app.sharing.*' --console=plain --max-workers=2
    if ($LASTEXITCODE -ne 0) { throw "Android source sharing test build failed: $LASTEXITCODE" }
    $shareReports = @(Get-ChildItem -LiteralPath 'app/build/test-results/testDebugUnitTest' -Filter 'TEST-com.elon.app.sharing*.xml')
    $shareTests = 0
    foreach ($shareReport in $shareReports) {
        [xml]$shareXml = Get-Content -LiteralPath $shareReport.FullName -Raw
        $shareSuite = $shareXml.testsuite
        $shareTests += [int]$shareSuite.tests
        if ([int]$shareSuite.failures -gt 0 -or [int]$shareSuite.errors -gt 0 -or [int]$shareSuite.skipped -gt 0) {
            throw "Android source sharing suite failed: $($shareSuite.name), failures=$($shareSuite.failures), errors=$($shareSuite.errors), skipped=$($shareSuite.skipped)"
        }
    }
    if ($shareReports.Count -ne 3 -or $shareTests -lt 7) { throw "Expected 3 sharing suites and at least 7 executed tests; found $($shareReports.Count) suites, $shareTests tests" }
    Write-Output "ANDROID_SOURCE_SHARING_TESTS=passed count=$shareTests"
} finally { Pop-Location }

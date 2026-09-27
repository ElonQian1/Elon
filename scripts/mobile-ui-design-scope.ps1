# Conservative fast-lane exclusions. Business semantics still require the V2 impact review.
function Test-ElonSystemicMobileDesignChange {
    param([string[]]$Paths = @())
    foreach ($path in $Paths) {
        $normalized = $path -replace '\\', '/'
        if ($normalized -match '^docs/design/mobile-tokens-v2\.json$' -or
            $normalized -match '^android/app/src/main/res/values(?:-night)?/(?:themes|mobile_design)\.xml$' -or
            $normalized -match '^scripts/(?:generate-mobile-tokens\.py|templates/mobile-design-v2\.css)$') {
            return $true
        }
    }
    return $false
}

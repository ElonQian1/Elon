# Public main-app identity. Both server aliases read the legacy storage key,
# including artifacts uploaded by publishers that predate the branding change.
$ElonAppBranding = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot '../server/src/assets/app_branding.json') | ConvertFrom-Json
foreach ($field in @('apkFileName', 'legacyApkFileName')) {
    if ([string]$ElonAppBranding.$field -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]*\.apk$') {
        throw "Invalid main-app APK filename: $field"
    }
}
$ElonApkFileName = [string]$ElonAppBranding.apkFileName
$ElonApkStorageFileName = [string]$ElonAppBranding.legacyApkFileName

function Get-ElonApkDownloadUrl {
    param([Parameter(Mandatory)][string]$ServerUrl)
    return "$($ServerUrl.TrimEnd('/'))/app/$ElonApkFileName"
}

Set-StrictMode -Version Latest

function New-ElonApkReleaseClaimBody {
    param(
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$BuilderId,
        [Parameter(Mandatory)][string]$BuilderLabel,
        [Parameter(Mandatory)][string]$CurrentVersionName,
        [Parameter(Mandatory)][int]$CurrentVersionCode,
        [Parameter(Mandatory)][int]$PublishedVersionCode,
        [int]$InstalledVersionCode = 0
    )

    $body = @{
        kind               = 'apk'
        sha                = $Sha
        builderId          = $BuilderId
        builderLabel       = $BuilderLabel
        bump               = 'patch'
        currentVersionName = $CurrentVersionName
        currentVersionCode = $CurrentVersionCode
    }
    if ($InstalledVersionCode -gt $PublishedVersionCode) {
        # Stable for retries, distinct from the ordinary same-SHA release batch.
        $body['batchId'] = "apk-device-floor-$Sha-$InstalledVersionCode"
        $body['stage'] = 'android_apk'
    }
    return $body
}

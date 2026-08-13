#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)][ValidateSet("health", "finances")][string]$FeatureKind,
    [switch]$UserConfirmedSensitiveFeature,
    [ValidateRange(20, 180)][int]$TimeoutSec = 90,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 89
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-feature-audit-policy.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

if (-not $UserConfirmedSensitiveFeature) {
    throw "Sensitive ChatGPT feature acceptance requires explicit user confirmation."
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$verificationCases = @{
    health = "supervised/feature_page/health"
    finances = "supervised/feature_page/finances"
}
$originPath = ""
$originPageKind = ""
$selectedFeature = $false

function Get-ObservedPath {
    param($State)

    $uri = $null
    if ([Uri]::TryCreate([string]$State.conversation.url, [UriKind]::Absolute, [ref]$uri)) {
        return $uri.AbsolutePath
    }
    return ""
}

function Wait-FeatureNavigation {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_list_features" -TimeoutSec 20 | Out-Null
        $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_get_navigation"
        $feature = @($navigation.features) | Where-Object {
            [string]$_.kind -eq $FeatureKind -and
                $_.requires_user_confirmation -eq $true
        } | Select-Object -First 1
        if ($null -ne $feature) { return $feature }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Sensitive ChatGPT feature is not currently available: $FeatureKind"
}

function Restore-Origin {
    if (-not $selectedFeature) { return }
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") `
        -TimeoutSec 8 -Label "restore sensitive feature origin" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description "sensitive feature origin restoration" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                [string]$state.page_kind -eq $originPageKind -and
                ((-not $originPath) -or (Get-ObservedPath -State $state) -eq $originPath)
        }.GetNewClosure() | Out-Null
    $selectedFeature = $false
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(15, $TimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ([string]$origin.view_mode -ne "web") {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = "official" } | Out-Null
        $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
            -TimeoutSec $TimeoutSec -InitialWaitSec 5
    }
    $originPath = Get-ObservedPath -State $origin
    $originPageKind = [string]$origin.page_kind

    $feature = Wait-FeatureNavigation
    $receipt = Invoke-ChatGptWebSmokeReceiptAction -Runtime $runtime `
        -Action "chatgpt_select_feature" -ExpectedAction "select_navigation" `
        -Arguments @{
            feature_id = [string]$feature.id
            user_confirmed = $true
        } -TimeoutSec $TimeoutSec
    if ($receipt.receipt.result.ok -ne $true) { throw "Sensitive feature selection failed." }
    $selectedFeature = $true

    $matrix = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description "sensitive feature page manifest" -Predicate {
            param($state)
            [string]$state.page_kind -eq "feature" -and
                [string]$state.ui_manifest.compatibility -eq "healthy" -and
                $state.bridge_state -eq "ready"
        }
    $capabilities = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_capability_matrix"
    $audit = Test-ChatGptWebFeatureMatrix -Matrix $capabilities
    if (-not $audit.passed) {
        throw "Sensitive feature page failed structural adaptation audit."
    }

    Restore-Origin
    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @($verificationCases[$FeatureKind]) `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
    [ordered]@{
        schema = "elon.chatgpt_web.sensitive_feature_page_smoke.v1"
        passed = $true
        feature_kind = $FeatureKind
        user_confirmed = $true
        origin_restored = $true
        control_count = [int]$audit.control_count
        private_content_emitted = $false
        mutations_invoked = 0
        sent_messages = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 6
    Write-Output "CHATGPT_WEB_SENSITIVE_FEATURE_PAGE_STATUS=passed kind=$FeatureKind"
} finally {
    try { Restore-Origin } catch { }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

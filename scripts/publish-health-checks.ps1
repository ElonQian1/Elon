function Resolve-ElonPublishCurl {
    $curl = Get-Command "curl.exe" -CommandType Application -ErrorAction SilentlyContinue
    if ($curl) { return $curl.Source }
    $curl = Get-Command "curl" -CommandType Application -ErrorAction SilentlyContinue
    if ($curl) { return $curl.Source }
    throw "curl is unavailable; cannot run publish smoke checks."
}

function Invoke-ElonPublishCurl {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $curl = Resolve-ElonPublishCurl
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $curl @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    $text = ($output -join "`n").Trim()
    if ($exitCode -ne 0) {
        throw "curl failed, exit=$exitCode, output=$text"
    }
    return $text
}

function Invoke-ElonPublishJsonGet {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [int]$TimeoutSec = 10
    )

    $raw = Invoke-ElonPublishCurl -Arguments @(
        "--noproxy", "*",
        "--silent", "--show-error", "--fail",
        "--max-time", [string]$TimeoutSec,
        $Uri
    )
    if ([string]::IsNullOrWhiteSpace($raw)) {
        throw "empty JSON response: $Uri"
    }
    return $raw | ConvertFrom-Json
}

function Invoke-ElonPublishTextGet {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [int]$TimeoutSec = 10
    )

    return Invoke-ElonPublishCurl -Arguments @(
        "--noproxy", "*",
        "--silent", "--show-error", "--fail",
        "--max-time", [string]$TimeoutSec,
        $Uri
    )
}

function Test-ElonPublishDownloadHead {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [int]$TimeoutSec = 15
    )

    $headers = Invoke-ElonPublishCurl -Arguments @(
        "--noproxy", "*",
        "--silent", "--show-error", "--fail",
        "--head",
        "--max-time", [string]$TimeoutSec,
        $Uri
    )
    if ($headers -notmatch "(?m)^HTTP/\S+\s+2\d\d") {
        throw "download endpoint did not return 2xx: $Uri"
    }
}

function Assert-ElonValue {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [AllowNull()][string]$Actual,
        [AllowNull()][string]$Expected
    )

    if ([string]::IsNullOrWhiteSpace($Expected)) { return }
    if ($Actual -ne $Expected) {
        throw "$Label mismatch: expected '$Expected', got '$Actual'"
    }
}

function Invoke-ElonServerPostDeploySmoke {
    param(
        [Parameter(Mandatory = $true)][string]$BaseUrl,
        [string]$ExpectedVersionName = "",
        [string]$ExpectedGitSha = "",
        [int]$TimeoutSec = 60,
        [int]$IntervalSec = 3
    )

    $base = $BaseUrl.TrimEnd([char]"/")
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastError = $null
    Write-Host "   运行后端发布 smoke（/health + /api/server/version）..." -ForegroundColor Gray
    while ((Get-Date) -lt $deadline) {
        try {
            $health = (Invoke-ElonPublishTextGet -Uri "$base/health" -TimeoutSec 10).Trim()
            if ($health -ne "OK") { throw "health returned '$health'" }
            $version = Invoke-ElonPublishJsonGet -Uri "$base/api/server/version" -TimeoutSec 10
            Assert-ElonValue -Label "versionName" -Actual ([string]$version.versionName) -Expected $ExpectedVersionName
            Assert-ElonValue -Label "gitSha" -Actual ([string]$version.gitSha) -Expected $ExpectedGitSha
            if ([string]$version.status -ne "ok") { throw "version status returned '$($version.status)'" }
            Write-Host "   ✅ 后端 smoke 通过: health=$health version=v$($version.versionName) sha=$($version.gitSha)" -ForegroundColor Green
            return $version
        } catch {
            $lastError = $_.Exception.Message
            Start-Sleep -Seconds $IntervalSec
        }
    }
    throw "后端发布 smoke 超时未通过：$lastError"
}

function Invoke-ElonNodeAgentPostUploadSmoke {
    param(
        [Parameter(Mandatory = $true)][string]$BaseUrl,
        [string]$ExpectedVersion = "",
        [string]$ExpectedGitSha = "",
        [string]$ExpectedWindowsSha256 = "",
        [string]$ExpectedLinuxSha256 = "",
        [string]$ExpectedWindowsClientSha256 = "",
        [string]$ExpectedWindowsInstallerSha256 = "",
        [switch]$IncludeRipgrep,
        [int]$TimeoutSec = 20
    )

    $base = $BaseUrl.TrimEnd([char]"/")
    Write-Host "  运行节点客户端下载 smoke（version manifest + download HEAD）..." -ForegroundColor Gray
    $version = Invoke-ElonPublishJsonGet -Uri "$base/api/node-agent/version" -TimeoutSec $TimeoutSec
    Assert-ElonValue -Label "node-agent version" -Actual ([string]$version.version) -Expected $ExpectedVersion
    Assert-ElonValue -Label "node-agent gitSha" -Actual ([string]$version.gitSha) -Expected $ExpectedGitSha
    Assert-ElonValue -Label "windows sha256" -Actual ([string]$version.sha256) -Expected $ExpectedWindowsSha256
    if (-not [string]::IsNullOrWhiteSpace($ExpectedLinuxSha256)) {
        Assert-ElonValue -Label "linux sha256" -Actual ([string]$version.linuxSha256) -Expected $ExpectedLinuxSha256
    }
    Assert-ElonValue -Label "windows client sha256" -Actual ([string]$version.windowsClientSha256) -Expected $ExpectedWindowsClientSha256
    Assert-ElonValue -Label "windows installer sha256" -Actual ([string]$version.windowsInstallerSha256) -Expected $ExpectedWindowsInstallerSha256

    $downloads = @(
        @{ Label = "linux"; Uri = "$base/api/node-agent/download/linux" },
        @{ Label = "windows"; Uri = "$base/api/node-agent/download/windows" },
        @{ Label = "windows-client"; Uri = "$base/api/node-agent/download/windows-client" }
        @{ Label = "windows-installer"; Uri = "$base/api/node-agent/download/windows-installer" }
    )
    if ($IncludeRipgrep) {
        $downloads += @{ Label = "ripgrep-windows"; Uri = "$base/api/node-agent/download/ripgrep-windows" }
    }

    foreach ($download in $downloads) {
        Test-ElonPublishDownloadHead -Uri $download.Uri -TimeoutSec $TimeoutSec
        Write-Host "  ✅ 下载端点可访问: $($download.Label)" -ForegroundColor Green
    }
    Write-Host "  ✅ 节点客户端 smoke 通过: version=$($version.version) sha=$($version.gitSha)" -ForegroundColor Green
    return $version
}

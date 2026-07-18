Set-StrictMode -Version Latest

function Invoke-NoProxyJson {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [string]$Method = "Get",
        [hashtable]$Headers = @{},
        [string]$Body = "",
        [int]$TimeoutSec = 15
    )

    $irmCommand = Get-Command Invoke-RestMethod -ErrorAction Stop
    $params = @{
        Uri = $Uri
        Method = $Method
        TimeoutSec = $TimeoutSec
    }
    if ($Headers.Count -gt 0) { $params["Headers"] = $Headers }
    if (-not [string]::IsNullOrWhiteSpace($Body)) {
        $params["Body"] = $Body
        $params["ContentType"] = "application/json"
    }
    if ($irmCommand.Parameters.ContainsKey("NoProxy")) {
        $params["NoProxy"] = $true
        return Invoke-RestMethod @params
    }

    $curl = Get-Command "curl.exe" -ErrorAction SilentlyContinue
    if ($curl) {
        $curlArgs = @(
            "--noproxy", "*", "--silent", "--show-error", "--fail",
            "--max-time", [string]$TimeoutSec, "-X", $Method
        )
        foreach ($key in $Headers.Keys) { $curlArgs += @("-H", "${key}: $($Headers[$key])") }
        if (-not [string]::IsNullOrWhiteSpace($Body)) {
            $curlArgs += @("-H", "Content-Type: application/json", "--data", $Body)
        }
        $curlArgs += $Uri
        $raw = & $curl.Source @curlArgs
        if ($LASTEXITCODE -ne 0) { throw "curl.exe 请求失败：$Uri" }
        if ([string]::IsNullOrWhiteSpace($raw)) { return $null }
        return $raw | ConvertFrom-Json
    }

    return Invoke-RestMethod @params
}

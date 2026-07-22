function Get-NodeConnection {
    param([int]$RetrySeconds = 0)
    $candidateUrls = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_ADMIN_URL)) {
        $candidateUrls.Add($env:ELON_NODE_ADMIN_URL.TrimEnd('/'))
    }
    if (-not [string]::IsNullOrWhiteSpace($script:LastNodeAdminUrl)) {
        $candidateUrls.Add($script:LastNodeAdminUrl)
    }
    $cachedUrl = Get-CachedNodeUrl
    if (-not [string]::IsNullOrWhiteSpace($cachedUrl)) { $candidateUrls.Add($cachedUrl) }
    $candidateUrls.Add('http://127.0.0.1:7799')
    foreach ($port in 7800..7819) { $candidateUrls.Add("http://127.0.0.1:$port") }

    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        $uniqueCandidates = @($candidateUrls | Select-Object -Unique)
        foreach ($candidateUrl in @($uniqueCandidates | Select-Object -First 4)) {
            try {
                $status = Invoke-Utf8JsonRequest -Method Get -Uri "$candidateUrl/api/status" `
                    -Headers @{ Origin = $candidateUrl } -TimeoutSec 1
                $token = [string](Get-ObjectField $status 'local_admin_token')
                $header = [string](Get-ObjectField $status 'local_admin_token_header')
                $version = [string](Get-ObjectField $status 'version')
                $supervisionStatus = Get-ObjectField $status 'desktop_supervision'
                if (-not [string]::IsNullOrWhiteSpace($token) -and
                    $header.ToLowerInvariant() -eq 'x-elon-local-admin-token' -and
                    -not [string]::IsNullOrWhiteSpace($version)) {
                    $script:LastNodeAdminUrl = $candidateUrl
                    Save-CachedNodeUrl $candidateUrl
                    return [pscustomobject]@{
                        BaseUrl = $candidateUrl; Header = $header; Token = $token; Version = $version
                        SupervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
                        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
                        ProbeMs = $timer.ElapsedMilliseconds; ProbeStrategy = 'cached_or_priority_bounded'
                    }
                }
            } catch { continue }
        }
        $parallel = Invoke-ParallelNodeProbe @($uniqueCandidates | Select-Object -Skip 4) 1200
        if ($null -ne $parallel) {
            $script:LastNodeAdminUrl = $parallel.BaseUrl
            Save-CachedNodeUrl $parallel.BaseUrl
            $parallel | Add-Member -NotePropertyName ProbeMs -NotePropertyValue $timer.ElapsedMilliseconds -Force
            $parallel | Add-Member -NotePropertyName ProbeStrategy -NotePropertyValue 'parallel_bounded_fallback' -Force
            return $parallel
        }
        if ($timer.Elapsed.TotalSeconds -lt $RetrySeconds) { Start-Sleep -Seconds 2 }
    } while ($timer.Elapsed.TotalSeconds -lt $RetrySeconds)
    throw 'No authorized Yilong PC node found on 127.0.0.1 ports 7799-7819.'
}

function Invoke-ParallelNodeProbe {
    param([string[]]$CandidateUrls, [int]$TimeoutMs = 1200)
    if (@($CandidateUrls).Count -eq 0) { return $null }
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.UseProxy = $false
    $client = [System.Net.Http.HttpClient]::new($handler)
    $client.Timeout = [TimeSpan]::FromMilliseconds($TimeoutMs)
    $pending = New-Object System.Collections.Generic.List[object]
    try {
        foreach ($candidateUrl in $CandidateUrls) {
            $request = [System.Net.Http.HttpRequestMessage]::new(
                [System.Net.Http.HttpMethod]::Get, "$candidateUrl/api/status")
            $null = $request.Headers.TryAddWithoutValidation('Origin', $candidateUrl)
            $pending.Add([pscustomobject]@{
                Url = $candidateUrl; Request = $request; Task = $client.SendAsync($request)
            }) | Out-Null
        }
        $tasks = [System.Threading.Tasks.Task[]]@($pending | ForEach-Object { $_.Task })
        try { $null = [System.Threading.Tasks.Task]::WaitAll($tasks, $TimeoutMs) } catch {}
        foreach ($item in $pending) {
            if ($item.Task.Status -ne [System.Threading.Tasks.TaskStatus]::RanToCompletion) { continue }
            $response = $item.Task.Result
            try {
                if (-not $response.IsSuccessStatusCode) { continue }
                [byte[]]$bytes = $response.Content.ReadAsByteArrayAsync().Result
                $status = Convert-JsonResponseBytes $bytes ([string]$response.Content.Headers.ContentType)
                $token = [string](Get-ObjectField $status 'local_admin_token')
                $header = [string](Get-ObjectField $status 'local_admin_token_header')
                $version = [string](Get-ObjectField $status 'version')
                $supervisionStatus = Get-ObjectField $status 'desktop_supervision'
                if (-not [string]::IsNullOrWhiteSpace($token) -and
                    $header.ToLowerInvariant() -eq 'x-elon-local-admin-token' -and
                    -not [string]::IsNullOrWhiteSpace($version)) {
                    return [pscustomobject]@{
                        BaseUrl = $item.Url; Header = $header; Token = $token; Version = $version
                        SupervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
                        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
                    }
                }
            } finally { $response.Dispose() }
        }
    } finally {
        foreach ($item in $pending) { $item.Request.Dispose() }
        $client.Dispose(); $handler.Dispose()
    }
    return $null
}

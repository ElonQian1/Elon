function Get-NodeConnection {
    param([int]$RetrySeconds = 5)
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
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

    $candidateBuildMs = $timer.ElapsedMilliseconds
    $priorityProbeMs = 0
    $fallbackProbeMs = 0
    $priorityAttemptCount = 0
    $fallbackCandidateCount = 0
    do {
        $uniqueCandidates = @($candidateUrls | Select-Object -Unique)
        $priorityTimer = [System.Diagnostics.Stopwatch]::StartNew()
        foreach ($candidateUrl in @($uniqueCandidates | Select-Object -First 1)) {
            $priorityAttemptCount++
            try {
                $status = Invoke-Utf8JsonRequest -Method Get -Uri "$candidateUrl/api/status" `
                    -Headers @{ Origin = $candidateUrl } -TimeoutSec 3
                $token = [string](Get-ObjectField $status 'local_admin_token')
                $header = [string](Get-ObjectField $status 'local_admin_token_header')
                $version = [string](Get-ObjectField $status 'version')
                $supervisionStatus = Get-ObjectField $status 'desktop_supervision'
                $desktopReviewBroker = Get-ObjectField $status 'desktop_review_broker'
                if (-not [string]::IsNullOrWhiteSpace($token) -and
                    $header.ToLowerInvariant() -eq 'x-elon-local-admin-token' -and
                    -not [string]::IsNullOrWhiteSpace($version)) {
                    $script:LastNodeAdminUrl = $candidateUrl
                    Save-CachedNodeUrl $candidateUrl
                    $priorityProbeMs += $priorityTimer.ElapsedMilliseconds
                    return [pscustomobject]@{
                        BaseUrl = $candidateUrl; Header = $header; Token = $token; Version = $version
                        SupervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
                        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
                        DesktopReviewBrokerAvailable = [bool](Get-ObjectField $desktopReviewBroker 'available')
                        DesktopReviewBrokerPipe = [string](Get-ObjectField $desktopReviewBroker 'pipe_name')
                        DesktopReviewBrokerStatus = $desktopReviewBroker
                        ProbeMs = $timer.ElapsedMilliseconds; ProbeStrategy = 'cached_or_priority_bounded'
                        ProbeTimings = [pscustomobject][ordered]@{
                            candidate_build_ms = $candidateBuildMs
                            priority_probe_ms = $priorityProbeMs
                            fallback_probe_ms = $fallbackProbeMs
                            total_ms = $timer.ElapsedMilliseconds
                            priority_attempt_count = $priorityAttemptCount
                            fallback_candidate_count = $fallbackCandidateCount
                            cache_candidate_present = -not [string]::IsNullOrWhiteSpace($cachedUrl)
                            persistent_cache_allowed = $script:PersistentNodeUrlCacheAllowed
                            result_phase = 'priority'
                        }
                    }
                }
            } catch { continue }
        }
        $priorityProbeMs += $priorityTimer.ElapsedMilliseconds
        $fallbackCandidates = @($uniqueCandidates | Select-Object -Skip 1)
        $fallbackCandidateCount += $fallbackCandidates.Count
        $fallbackTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $parallel = Invoke-ParallelNodeProbe $fallbackCandidates 2500
        $fallbackProbeMs += $fallbackTimer.ElapsedMilliseconds
        if ($null -ne $parallel) {
            $script:LastNodeAdminUrl = $parallel.BaseUrl
            Save-CachedNodeUrl $parallel.BaseUrl
            $parallel | Add-Member -NotePropertyName ProbeMs -NotePropertyValue $timer.ElapsedMilliseconds -Force
            $parallel | Add-Member -NotePropertyName ProbeStrategy -NotePropertyValue 'parallel_bounded_fallback' -Force
            $parallel | Add-Member -NotePropertyName ProbeTimings -NotePropertyValue ([pscustomobject][ordered]@{
                candidate_build_ms = $candidateBuildMs
                priority_probe_ms = $priorityProbeMs
                fallback_probe_ms = $fallbackProbeMs
                total_ms = $timer.ElapsedMilliseconds
                priority_attempt_count = $priorityAttemptCount
                fallback_candidate_count = $fallbackCandidateCount
                cache_candidate_present = -not [string]::IsNullOrWhiteSpace($cachedUrl)
                persistent_cache_allowed = $script:PersistentNodeUrlCacheAllowed
                result_phase = 'parallel_fallback'
            }) -Force
            return $parallel
        }
        if ($timer.Elapsed.TotalSeconds -lt $RetrySeconds) { Start-Sleep -Milliseconds 100 }
    } while ($timer.Elapsed.TotalSeconds -lt $RetrySeconds)
    throw 'No authorized Yilong PC node found on 127.0.0.1 ports 7799-7819.'
}

function Invoke-ParallelNodeProbe {
    param([string[]]$CandidateUrls, [int]$TimeoutMs = 2500)
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
                $desktopReviewBroker = Get-ObjectField $status 'desktop_review_broker'
                if (-not [string]::IsNullOrWhiteSpace($token) -and
                    $header.ToLowerInvariant() -eq 'x-elon-local-admin-token' -and
                    -not [string]::IsNullOrWhiteSpace($version)) {
                    return [pscustomobject]@{
                        BaseUrl = $item.Url; Header = $header; Token = $token; Version = $version
                        SupervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
                        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
                        DesktopReviewBrokerAvailable = [bool](Get-ObjectField $desktopReviewBroker 'available')
                        DesktopReviewBrokerPipe = [string](Get-ObjectField $desktopReviewBroker 'pipe_name')
                        DesktopReviewBrokerStatus = $desktopReviewBroker
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

function ConvertTo-NodeConnectionCandidate {
    param([string]$CandidateUrl, [object]$Status, [string]$RequestedAction)
    $token = [string](Get-ObjectField $Status 'local_admin_token')
    $header = [string](Get-ObjectField $Status 'local_admin_token_header')
    $version = [string](Get-ObjectField $Status 'version')
    $supervisionStatus = Get-ObjectField $Status 'desktop_supervision'
    $desktopReviewBroker = Get-ObjectField $Status 'desktop_review_broker'
    $loggedIn = (Get-ObjectField $Status 'logged_in') -eq $true
    $userTokenConfigured = (Get-ObjectField $Status 'user_token_configured') -eq $true
    $supervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
    $taskAuthorized = $loggedIn -and $userTokenConfigured -and
        $supervisionProtocol -eq $script:SupervisionProtocol
    if ([string]::IsNullOrWhiteSpace($token) -or
        $header.ToLowerInvariant() -ne 'x-elon-local-admin-token' -or
        [string]::IsNullOrWhiteSpace($version) -or
        ($RequestedAction -ne 'Probe' -and -not $taskAuthorized)) {
        return $null
    }
    return [pscustomobject]@{
        BaseUrl = $CandidateUrl; Header = $header; Token = $token; Version = $version
        LoggedIn = $loggedIn; UserTokenConfigured = $userTokenConfigured
        TaskAuthorized = $taskAuthorized
        AgentId = [string](Get-ObjectField $Status 'agent_id')
        OwnerUserId = [string](Get-ObjectField $Status 'owner_user_id')
        SupervisionProtocol = $supervisionProtocol
        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
        DesktopReviewBrokerAvailable = [bool](Get-ObjectField $desktopReviewBroker 'available')
        DesktopReviewBrokerPipe = [string](Get-ObjectField $desktopReviewBroker 'pipe_name')
        DesktopReviewBrokerStatus = $desktopReviewBroker
    }
}

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
                $connection = ConvertTo-NodeConnectionCandidate $candidateUrl $status $Action
                if ($null -ne $connection) {
                    $script:LastNodeAdminUrl = $candidateUrl
                    Save-CachedNodeUrl $candidateUrl
                    $priorityProbeMs += $priorityTimer.ElapsedMilliseconds
                    $connection | Add-Member -NotePropertyName ProbeMs -NotePropertyValue $timer.ElapsedMilliseconds -Force
                    $connection | Add-Member -NotePropertyName ProbeStrategy -NotePropertyValue 'cached_or_priority_bounded' -Force
                    $connection | Add-Member -NotePropertyName ProbeTimings -NotePropertyValue ([pscustomobject][ordered]@{
                        candidate_build_ms = $candidateBuildMs
                        priority_probe_ms = $priorityProbeMs
                        fallback_probe_ms = $fallbackProbeMs
                        total_ms = $timer.ElapsedMilliseconds
                        priority_attempt_count = $priorityAttemptCount
                        fallback_candidate_count = $fallbackCandidateCount
                        cache_candidate_present = -not [string]::IsNullOrWhiteSpace($cachedUrl)
                        persistent_cache_allowed = $script:PersistentNodeUrlCacheAllowed
                        result_phase = 'priority'
                    }) -Force
                    return $connection
                }
            } catch { continue }
        }
        $priorityProbeMs += $priorityTimer.ElapsedMilliseconds
        $fallbackCandidates = @($uniqueCandidates | Select-Object -Skip 1)
        $fallbackCandidateCount += $fallbackCandidates.Count
        $fallbackTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $parallel = Invoke-ParallelNodeProbe $fallbackCandidates 2500 $Action
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
    param(
        [string[]]$CandidateUrls,
        [int]$TimeoutMs = 2500,
        [string]$RequestedAction = 'Probe'
    )
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
                $connection =
                    ConvertTo-NodeConnectionCandidate $item.Url $status $RequestedAction
                if ($null -ne $connection) { return $connection }
            } finally { $response.Dispose() }
        }
    } finally {
        foreach ($item in $pending) { $item.Request.Dispose() }
        $client.Dispose(); $handler.Dispose()
    }
    return $null
}

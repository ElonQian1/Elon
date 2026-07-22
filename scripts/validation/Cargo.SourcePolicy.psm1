Import-Module "$PSScriptRoot\Cargo.Diagnostics.psm1" -Force -DisableNameChecking

function Get-CargoSourcePolicy {
    param([string]$Path = (Join-Path $PSScriptRoot 'cargo-sources.json'))
    try { $policy = Get-Content -Raw -LiteralPath $Path -Encoding UTF8 | ConvertFrom-Json } catch { throw "Invalid Cargo source policy: $Path. $($_.Exception.Message)" }
    if ($policy.schema -ne 'elon.cargo_sources.v1' -or @($policy.sources).Count -eq 0) { throw "Unsupported or empty Cargo source policy: $Path" }
    $ids = @{}
    foreach ($source in @($policy.sources)) {
        if ([string]::IsNullOrWhiteSpace($source.id) -or $ids.ContainsKey([string]$source.id)) { throw "Cargo source ids must be unique and non-empty: $Path" }
        $ids[[string]$source.id] = $true
        Assert-CargoSparseHttpsUri -Value ([string]$source.index) -Label "source '$($source.id)' index" | Out-Null
        Assert-CargoHttpsUri -Value ([string]$source.evidence) -Label "source '$($source.id)' evidence" | Out-Null
        if (@($source.operator_domains).Count -eq 0) { throw "Cargo source '$($source.id)' has no operator_domains." }
    }
    return $policy
}

function Assert-CargoHttpsUri {
    param([Parameter(Mandatory)][string]$Value, [string]$Label = 'URI')
    $uri = $null
    if (-not [Uri]::TryCreate($Value, [UriKind]::Absolute, [ref]$uri) -or $uri.Scheme -ne 'https' -or -not [string]::IsNullOrEmpty($uri.UserInfo)) {
        throw "$Label must be an absolute HTTPS URI without embedded credentials: $Value"
    }
    return $uri
}

function Assert-CargoSparseHttpsUri {
    param([Parameter(Mandatory)][string]$Value, [string]$Label = 'sparse index')
    if (-not $Value.StartsWith('sparse+https://', [StringComparison]::OrdinalIgnoreCase)) { throw "$Label must use sparse+https: $Value" }
    $https = 'https://' + $Value.Substring('sparse+https://'.Length)
    $uri = Assert-CargoHttpsUri -Value $https -Label $Label
    if (-not $uri.AbsoluteUri.EndsWith('/')) { throw "$Label must end with '/': $Value" }
    return $uri
}

function Test-CargoHostWithinDomains {
    param([Parameter(Mandatory)][string]$Host, [string[]]$Domains = @())
    $normalized = $Host.TrimEnd('.').ToLowerInvariant()
    foreach ($domain in @($Domains)) {
        $suffix = ([string]$domain).Trim().TrimStart('.').TrimEnd('.').ToLowerInvariant()
        if ($suffix -and ($normalized -eq $suffix -or $normalized.EndsWith(".$suffix"))) { return $true }
    }
    return $false
}

function Test-CargoRedirectTarget {
    param([Parameter(Mandatory)][Uri]$Uri,[string[]]$AllowedDomains=@())
    if($Uri.Scheme -ne 'https' -or -not [string]::IsNullOrEmpty($Uri.UserInfo)){return $false}
    if(@($AllowedDomains).Count -gt 0 -and -not(Test-CargoHostWithinDomains $Uri.Host $AllowedDomains)){return $false}
    return $true
}

function Invoke-CargoHttpRequest {
    param(
        [Parameter(Mandatory)][Uri]$Uri,
        [int]$TimeoutSeconds = 8,
        [int]$MaximumRedirects = 2,
        [string[]]$AllowedRedirectDomains = @(),
        [switch]$RangeOnly,
        [scriptblock]$Invoker
    )
    if ($Invoker) { return & $Invoker $Uri $TimeoutSeconds $MaximumRedirects $AllowedRedirectDomains ([bool]$RangeOnly) }
    Add-Type -AssemblyName System.Net.Http
    $handler = New-Object System.Net.Http.HttpClientHandler
    $handler.AllowAutoRedirect = $false
    $client = New-Object System.Net.Http.HttpClient($handler)
    $client.Timeout = [TimeSpan]::FromSeconds([Math]::Max(1, $TimeoutSeconds))
    try {
        $current = $Uri
        for ($redirects = 0; $redirects -le $MaximumRedirects; $redirects++) {
            if ($current.Scheme -ne 'https' -or -not [string]::IsNullOrEmpty($current.UserInfo)) {
                return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_UNSAFE_REDIRECT';message='Redirect target is not credential-free HTTPS.';uri=$current.AbsoluteUri;status_code=$null}
            }
            $request = New-Object System.Net.Http.HttpRequestMessage([System.Net.Http.HttpMethod]::Get, $current)
            if ($RangeOnly) { $request.Headers.Range = New-Object System.Net.Http.Headers.RangeHeaderValue(0,0) }
            try { $response = $client.SendAsync($request).GetAwaiter().GetResult() } catch {
                $message = $_.Exception.ToString()
                $code = if ($message -match 'timed out|TaskCanceled') {'CARGO_SOURCE_TIMEOUT'} elseif ($message -match 'certificate|SSL|TLS') {'CARGO_TLS_FAILURE'} elseif ($message -match 'resolve|Name or service') {'CARGO_DNS_FAILURE'} else {'CARGO_SOURCE_HTTP_FAILURE'}
                return [pscustomobject]@{ok=$false;code=$code;message=$message;uri=$current.AbsoluteUri;status_code=$null}
            }
            try {
                $status = [int]$response.StatusCode
                if ($status -ge 300 -and $status -lt 400) {
                    if ($redirects -ge $MaximumRedirects -or $null -eq $response.Headers.Location) {
                        return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_REDIRECT_LIMIT';message='Redirect limit exceeded or Location missing.';uri=$current.AbsoluteUri;status_code=$status}
                    }
                    $next = if ($response.Headers.Location.IsAbsoluteUri) { $response.Headers.Location } else { [Uri]::new($current, $response.Headers.Location) }
                    if (-not (Test-CargoRedirectTarget $next $AllowedRedirectDomains)) {
                        return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_UNSAFE_REDIRECT';message='Redirect left the approved HTTPS operator domains.';uri=$next.AbsoluteUri;status_code=$status}
                    }
                    $current = $next
                    continue
                }
                $body = if ($RangeOnly) { '' } else { $response.Content.ReadAsStringAsync().GetAwaiter().GetResult() }
                return [pscustomobject]@{ok=($status -ge 200 -and $status -lt 300);code=if($status -ge 200 -and $status -lt 300){'OK'}else{'CARGO_SOURCE_HTTP_STATUS'};message="HTTP $status";uri=$current.AbsoluteUri;status_code=$status;body=$body;headers=$response.Headers.ToString()}
            } finally { $response.Dispose() }
        }
    } finally { $client.Dispose(); $handler.Dispose() }
}

function Get-CargoLockProbePackage {
    param([Parameter(Mandatory)][string]$CargoLockPath)
    if (-not (Test-Path -LiteralPath $CargoLockPath)) { throw "Cargo.lock is required for source validation: $CargoLockPath" }
    $current = @{}
    foreach ($line in Get-Content -LiteralPath $CargoLockPath -Encoding UTF8) {
        if ($line -match '^\[\[package\]\]') {
            if ($current.source -like 'registry+*' -and $current.checksum) { return [pscustomobject]$current }
            $current = @{}
        } elseif ($line -match '^(name|version|source|checksum)\s*=\s*"([^"]+)"') {
            $current[$Matches[1]] = $Matches[2]
        }
    }
    if ($current.source -like 'registry+*' -and $current.checksum) { return [pscustomobject]$current }
    throw "Cargo.lock has no registry package with a checksum: $CargoLockPath"
}

function Get-CargoCratePrefix {
    param([Parameter(Mandatory)][string]$Name)
    $lower = $Name.ToLowerInvariant()
    if ($lower.Length -eq 1) { return '1' }
    if ($lower.Length -eq 2) { return '2' }
    if ($lower.Length -eq 3) { return "3/$($lower.Substring(0,1))" }
    return "$($lower.Substring(0,2))/$($lower.Substring(2,2))"
}

function Resolve-CargoDownloadUri {
    param([Parameter(Mandatory)][string]$Template, [Parameter(Mandatory)]$Package)
    $uri = Assert-CargoHttpsUri $Template 'registry config.json dl'
    $value = $uri.AbsoluteUri.TrimEnd('/')
    $markers = @('{crate}','{version}','{prefix}','{lowerprefix}','{sha256-checksum}')
    $hasMarker = $false; foreach ($marker in $markers) { if ($value.Contains($marker)) { $hasMarker=$true } }
    if ($hasMarker) {
        $prefix = Get-CargoCratePrefix ([string]$Package.name)
        $value = $value.Replace('{crate}',[Uri]::EscapeDataString([string]$Package.name)).Replace('{version}',[Uri]::EscapeDataString([string]$Package.version)).Replace('{prefix}',$prefix).Replace('{lowerprefix}',$prefix.ToLowerInvariant()).Replace('{sha256-checksum}',[string]$Package.checksum)
    } else {
        $value += "/$([Uri]::EscapeDataString([string]$Package.name))/$([Uri]::EscapeDataString([string]$Package.version))/download"
    }
    return Assert-CargoHttpsUri $value 'resolved crate download'
}

function Test-CargoSourceEndpoint {
    param(
        [Parameter(Mandatory)]$Source,
        [Parameter(Mandatory)][string]$CargoLockPath,
        [int]$TimeoutSeconds = 8,
        [int]$MaximumRedirects = 2,
        [scriptblock]$HttpInvoker
    )
    try {
        $indexUri = Assert-CargoSparseHttpsUri ([string]$Source.index) "source '$($Source.id)' index"
        $configUri = [Uri]::new($indexUri, 'config.json')
        $domains = @($Source.operator_domains) + @($indexUri.Host)
        $configResponse = Invoke-CargoHttpRequest $configUri $TimeoutSeconds $MaximumRedirects $domains -Invoker $HttpInvoker
        if (-not $configResponse.ok) { return [pscustomobject]@{ok=$false;code='CARGO_INDEX_FAILURE';cause_code=$configResponse.code;stage='index_config';message=$configResponse.message;uri=$configResponse.uri;status_code=$configResponse.status_code} }
        try { $config = $configResponse.body | ConvertFrom-Json } catch { return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_INVALID_CONFIG_JSON';stage='index_config';message=$_.Exception.Message;uri=$configUri.AbsoluteUri;status_code=$configResponse.status_code} }
        if ([string]::IsNullOrWhiteSpace([string]$config.dl)) { return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_INVALID_CONFIG_JSON';stage='index_config';message='config.json is missing dl.';uri=$configUri.AbsoluteUri;status_code=$configResponse.status_code} }
        $package = Get-CargoLockProbePackage $CargoLockPath
        $downloadUri = Resolve-CargoDownloadUri ([string]$config.dl) $package
        $downloadDomains = @($downloadUri.Host)
        $downloadResponse = Invoke-CargoHttpRequest $downloadUri $TimeoutSeconds $MaximumRedirects $downloadDomains -RangeOnly -Invoker $HttpInvoker
        if (-not $downloadResponse.ok) { return [pscustomobject]@{ok=$false;code='CARGO_CRATE_DOWNLOAD_FAILURE';cause_code=$downloadResponse.code;stage='crate_download';message=$downloadResponse.message;uri=$downloadResponse.uri;status_code=$downloadResponse.status_code} }
        return [pscustomobject]@{ok=$true;code='CARGO_SOURCE_HEALTHY';message='config.json and Cargo.lock-derived download endpoint passed.';uri=$indexUri.AbsoluteUri;status_code=200;download_uri=$downloadUri.AbsoluteUri;package="$($package.name)@$($package.version)";checksum=$package.checksum}
    } catch {
        return [pscustomobject]@{ok=$false;code='CARGO_SOURCE_POLICY_REJECTED';stage='source_policy';message=$_.Exception.Message;uri=[string]$Source.index;status_code=$null}
    }
}

function Get-CargoSourceHealthState {
    param([Parameter(Mandatory)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return [ordered]@{schema='elon.cargo_source_health.v1';sources=[ordered]@{}} }
    try { $state = Get-Content -Raw -LiteralPath $Path -Encoding UTF8 | ConvertFrom-Json } catch { return [ordered]@{schema='elon.cargo_source_health.v1';sources=[ordered]@{}} }
    if ($state.schema -ne 'elon.cargo_source_health.v1') { return [ordered]@{schema='elon.cargo_source_health.v1';sources=[ordered]@{}} }
    return $state
}

function Test-CargoSourceCircuitOpen {
    param([Parameter(Mandatory)]$State, [Parameter(Mandatory)][string]$SourceId)
    $record = $State.sources.$SourceId
    if (-not $record -or -not $record.circuit_until_utc) { return $false }
    try { return [DateTime]::Parse([string]$record.circuit_until_utc).ToUniversalTime() -gt [DateTime]::UtcNow } catch { return $false }
}

function Test-CargoSourceHealthFresh {
    param([Parameter(Mandatory)]$State, [Parameter(Mandatory)][string]$SourceId, [int]$TtlSeconds = 600)
    $record = $State.sources.$SourceId
    if (-not $record -or $record.status -ne 'healthy' -or -not $record.checked_utc) { return $false }
    try { return ([DateTime]::UtcNow - [DateTime]::Parse([string]$record.checked_utc).ToUniversalTime()).TotalSeconds -le $TtlSeconds } catch { return $false }
}

function Update-CargoSourceHealthState {
    param([Parameter(Mandatory)][string]$Path,[Parameter(Mandatory)][string]$SourceId,[Parameter(Mandatory)][bool]$Healthy,[string]$Code,[int]$FailureThreshold=2,[int]$CircuitOpenSeconds=300)
    New-Item -ItemType Directory -Force -Path (Split-Path $Path -Parent) | Out-Null
    $lockPath="$Path.lock";$stream=$null;$deadline=[DateTime]::UtcNow.AddSeconds(5)
    while($null -eq $stream){try{$stream=[IO.File]::Open($lockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)}catch{if([DateTime]::UtcNow -ge $deadline){throw "Timed out locking Cargo source health state: $Path"};Start-Sleep -Milliseconds 50}}
    try{
        $state = Get-CargoSourceHealthState $Path
        $prior = $state.sources.$SourceId
        $failures = if ($Healthy) { 0 } elseif ($prior) { [int]$prior.consecutive_failures + 1 } else { 1 }
        $record = [ordered]@{status=if($Healthy){'healthy'}else{'failed'};checked_utc=[DateTime]::UtcNow.ToString('o');code=$Code;consecutive_failures=$failures;circuit_until_utc=if(-not $Healthy -and $failures -ge $FailureThreshold){[DateTime]::UtcNow.AddSeconds($CircuitOpenSeconds).ToString('o')}else{$null}}
        if ($state.sources -is [System.Collections.IDictionary]) { $state.sources[$SourceId] = $record } else { $state.sources | Add-Member -NotePropertyName $SourceId -NotePropertyValue ([pscustomobject]$record) -Force }
        $temporary = "$Path.$PID.tmp"; $state | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $temporary -Encoding UTF8; Move-Item -LiteralPath $temporary -Destination $Path -Force
        return [pscustomobject]$record
    } finally {$stream.Dispose()}
}

function New-CargoSourceHomeConfig {
    param([Parameter(Mandatory)]$Source,[Parameter(Mandatory)][string]$CargoHome,[int]$HttpTimeoutSeconds=30)
    New-Item -ItemType Directory -Force -Path $CargoHome | Out-Null
    $index = [string]$Source.index
    $lines = @('[net]','retry = 0','git-fetch-with-cli = true','', '[http]',"timeout = $HttpTimeoutSeconds",'', '[registries.crates-io]','protocol = "sparse"')
    if ($index -ne 'sparse+https://index.crates.io/') {
        $lines += @('', '[source.crates-io]', 'replace-with = "elon-managed"', '', '[source.elon-managed]', "registry = `"$index`"")
    }
    $configPath = Join-Path $CargoHome 'config.toml'
    ($lines -join [Environment]::NewLine) + [Environment]::NewLine | Set-Content -LiteralPath $configPath -Encoding UTF8 -NoNewline
    return $configPath
}

function Test-CargoCandidateEvidence {
    param([Parameter(Mandatory)][string]$Index,[Parameter(Mandatory)][string]$Evidence,[int]$TimeoutSeconds=8,[int]$MaximumRedirects=2,[scriptblock]$HttpInvoker)
    try {
        $indexUri=Assert-CargoSparseHttpsUri $Index 'candidate index'; $evidenceUri=Assert-CargoHttpsUri $Evidence 'candidate evidence'
        $domains=@($indexUri.Host,$evidenceUri.Host)
        if (-not (Test-CargoHostWithinDomains $indexUri.Host @($evidenceUri.Host)) -and -not (Test-CargoHostWithinDomains $evidenceUri.Host @($indexUri.Host))) {
            throw 'Candidate evidence must use the same host or a direct parent/subdomain of the index host.'
        }
        $response=Invoke-CargoHttpRequest $evidenceUri $TimeoutSeconds $MaximumRedirects $domains -Invoker $HttpInvoker
        if(-not $response.ok){return $response}
        if(([string]$response.body) -notmatch [regex]::Escape($indexUri.Host)){return [pscustomobject]@{ok=$false;code='CARGO_CANDIDATE_EVIDENCE_MISMATCH';message='Operator evidence does not name the candidate index host.';uri=$evidenceUri.AbsoluteUri;status_code=$response.status_code}}
        return [pscustomobject]@{ok=$true;code='CARGO_CANDIDATE_EVIDENCE_VALID';message='HTTPS operator evidence names the candidate index host.';uri=$evidenceUri.AbsoluteUri;status_code=$response.status_code}
    } catch { return [pscustomobject]@{ok=$false;code='CARGO_CANDIDATE_EVIDENCE_REJECTED';message=$_.Exception.Message;uri=$Evidence;status_code=$null} }
}

Export-ModuleMember -Function Get-CargoSourcePolicy, Assert-CargoHttpsUri, Assert-CargoSparseHttpsUri, Test-CargoHostWithinDomains, Test-CargoRedirectTarget, Invoke-CargoHttpRequest, Get-CargoLockProbePackage, Resolve-CargoDownloadUri, Test-CargoSourceEndpoint, Get-CargoSourceHealthState, Test-CargoSourceCircuitOpen, Test-CargoSourceHealthFresh, Update-CargoSourceHealthState, New-CargoSourceHomeConfig, Test-CargoCandidateEvidence

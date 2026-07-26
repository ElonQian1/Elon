function New-CargoDiagnostic {
    param(
        [Parameter(Mandatory)][string]$Code,
        [Parameter(Mandatory)][string]$Stage,
        [Parameter(Mandatory)][bool]$Retryable,
        [string]$Message,
        [string[]]$Evidence = @()
    )
    [pscustomobject]@{
        schema = 'elon.cargo_diagnostic.v1'
        code = $Code
        stage = $Stage
        retryable = $Retryable
        message = $Message
        evidence = @($Evidence | Where-Object { $_ } | Select-Object -First 12)
    }
}

function Get-CargoFailureDiagnostic {
    param(
        [string[]]$Lines = @(),
        [string]$Stage = 'cargo',
        [int]$ExitCode = 1,
        [switch]$TimedOut
    )
    $text = (@($Lines) -join "`n")
    $evidence = @($Lines | Where-Object { $_ -and $_.Trim() } | Select-Object -Last 12)
    if ($TimedOut) {
        return New-CargoDiagnostic 'CARGO_NETWORK_TIMEOUT' $Stage $true 'Cargo network stage exceeded its bounded timeout.' $evidence
    }
    if ($ExitCode -eq 0) {
        return New-CargoDiagnostic 'CARGO_OK' $Stage $false 'Cargo completed successfully.' @()
    }

    $rules = @(
        @{ code='CARGO_DISK_CRITICAL'; retry=$false; pattern='No space left on device|not enough space|os error 112|disk full|insufficient disk' },
        @{ code='CARGO_CACHE_LOCKED'; retry=$false; pattern='Blocking waiting for file lock|failed to acquire package cache lock|cache lock|lock file.*busy|Timed out waiting for Rust cache lock' },
        @{ code='CARGO_OFFLINE_MISSING'; retry=$true; pattern='--offline was specified|no matching package named .+ found|failed to download .+offline|attempting to make an HTTP request, but --offline' },
        @{ code='CARGO_TEST_FAILURE'; retry=$false; pattern='test failed, to rerun pass|test result: FAILED|(?m)^\s*failures:\s*$' },
        @{ code='CARGO_DNS_FAILURE'; retry=$true; pattern='Could not resolve host|failed to lookup address|Name or service not known|Temporary failure in name resolution|dns error' },
        @{ code='CARGO_TLS_FAILURE'; retry=$true; pattern='certificate|\bSSL\b|\bTLS\b|schannel|peer certificate|UnknownIssuer|InitializeSecurityContext' },
        @{ code='CARGO_PROXY_FAILURE'; retry=$true; pattern='proxy authentication|proxy error|proxy connect|407 Proxy Authentication|CONNECT tunnel failed|proxy URL' },
        @{ code='CARGO_GIT_DEPENDENCY_FAILURE'; retry=$false; pattern='Unable to update git\+|failed to clone into|git fetch|revision [0-9a-f]+ not found.*git|network failure seems to have happened.*git' },
        @{ code='CARGO_CRATE_DOWNLOAD_FAILURE'; retry=$true; pattern='failed to download|download of .+ failed|failed to get successful HTTP response.*\.crate|failed to unpack package' },
        @{ code='CARGO_INDEX_FAILURE'; retry=$true; pattern='failed to query replaced source registry|failed to update registry|failed to fetch .*index|config\.json|registry index|spurious network error' },
        @{ code='RUST_COMPILE_ERROR'; retry=$false; pattern='could not compile|error\[E[0-9]+\]|error: aborting due to|linking with .+ failed|test result: FAILED' }
    )
    foreach ($rule in $rules) {
        if ($text -match $rule.pattern) {
            return New-CargoDiagnostic $rule.code $Stage ([bool]$rule.retry) "Cargo failure classified as $($rule.code)." $evidence
        }
    }
    return New-CargoDiagnostic 'CARGO_UNKNOWN_FAILURE' $Stage $false 'Cargo failed without a safely retryable classification.' $evidence
}

function Get-CargoProbeDiagnostic {
    param([Parameter(Mandatory)]$Probe, [string]$Stage = 'source_probe')
    if($Probe.stage){$Stage=[string]$Probe.stage}
    if ([bool]$Probe.ok) { return New-CargoDiagnostic 'CARGO_SOURCE_HEALTHY' $Stage $false 'Source endpoint validation passed.' @() }
    $code = if ($Probe.code) { [string]$Probe.code } else { 'CARGO_SOURCE_PROBE_FAILED' }
    $retryable = $code -match 'TIMEOUT|HTTP|DNS|TLS|PROBE'
    $items = @()
    if ($Probe.message) { $items += [string]$Probe.message }
    if ($Probe.cause_code) { $items += "cause_code=$($Probe.cause_code)" }
    if ($Probe.uri) { $items += "uri=$($Probe.uri)" }
    if ($null -ne $Probe.status_code) { $items += "status=$($Probe.status_code)" }
    return New-CargoDiagnostic $code $Stage $retryable ([string]$Probe.message) $items
}

function Write-CargoMachineStatus {
    param([Parameter(Mandatory)]$Diagnostic)
    Write-Host "CARGO_STATUS_CODE=$($Diagnostic.code)"
    Write-Host "CARGO_STATUS_STAGE=$($Diagnostic.stage)"
    Write-Host "CARGO_STATUS_RETRYABLE=$(([bool]$Diagnostic.retryable).ToString().ToLowerInvariant())"
    Write-Host ('CARGO_STATUS_JSON=' + ($Diagnostic | ConvertTo-Json -Depth 8 -Compress))
}

Export-ModuleMember -Function New-CargoDiagnostic, Get-CargoFailureDiagnostic, Get-CargoProbeDiagnostic, Write-CargoMachineStatus

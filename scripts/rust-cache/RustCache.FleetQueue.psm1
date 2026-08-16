Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Fleet.psm1" -Force -DisableNameChecking

$script:FleetEnvelopeSchema = "elon.rust_cache.fleet_envelope.v1"
$script:FleetReportSchema = "elon.rust_cache.fleet_report.v1"
$script:MaxFleetReportBytes = 1MB
$script:Utf8NoBom = [System.Text.UTF8Encoding]::new($false)

function Get-RustCacheFleetPayloadHash {
    param([Parameter(Mandatory)][string]$Payload)

    $bytes = $script:Utf8NoBom.GetBytes($Payload)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function New-RustCacheFleetEnvelope {
    param([Parameter(Mandatory)]$Report)

    if ([string]$Report.schema -ne $script:FleetReportSchema) {
        throw "Unsupported fleet report schema: $($Report.schema)"
    }
    $nodeId = [string]$Report.node.node_id
    if ([string]::IsNullOrWhiteSpace($nodeId)) {
        throw "A fleet envelope requires an explicit platform NodeId."
    }
    Assert-RustCacheFleetNodeId -NodeId $nodeId
    if ([bool]$Report.destructive_actions_taken) {
        throw "A fleet envelope cannot carry a report that performed destructive actions."
    }

    $reportJson = $Report | ConvertTo-Json -Depth 10 -Compress
    $reportBytes = $script:Utf8NoBom.GetByteCount($reportJson)
    if ($reportBytes -gt $script:MaxFleetReportBytes) {
        throw "Fleet report exceeds the 1 MiB envelope limit."
    }

    [pscustomobject][ordered]@{
        schema = $script:FleetEnvelopeSchema
        envelope_id = [Guid]::NewGuid().ToString("N")
        created_at_utc = [DateTime]::UtcNow.ToString("o")
        node_id = $nodeId
        report = [pscustomobject][ordered]@{
            schema = $script:FleetReportSchema
            content_type = "application/json"
            content_sha256 = Get-RustCacheFleetPayloadHash -Payload $reportJson
            byte_length = $reportBytes
            json = $reportJson
        }
        security = [pscustomobject][ordered]@{
            receiver_must_authenticate_node = $true
            destructive_actions_authorized = $false
            absolute_paths_included = $false
            secrets_included = $false
        }
    }
}

function Test-RustCacheFleetEnvelope {
    param([Parameter(Mandatory)]$Envelope)

    $errors = [System.Collections.Generic.List[string]]::new()
    if ([string]$Envelope.schema -ne $script:FleetEnvelopeSchema) {
        $errors.Add("unsupported-envelope-schema")
    }
    $nodeId = [string]$Envelope.node_id
    try {
        if ([string]::IsNullOrWhiteSpace($nodeId)) { throw "missing" }
        Assert-RustCacheFleetNodeId -NodeId $nodeId
    } catch {
        $errors.Add("invalid-node-id")
    }
    $reportJson = [string]$Envelope.report.json
    $reportBytes = $script:Utf8NoBom.GetByteCount($reportJson)
    if ($reportBytes -gt $script:MaxFleetReportBytes) {
        $errors.Add("report-too-large")
    }
    if ([string]$Envelope.report.content_sha256 -ne (Get-RustCacheFleetPayloadHash -Payload $reportJson)) {
        $errors.Add("report-hash-mismatch")
    }

    $report = $null
    try { $report = $reportJson | ConvertFrom-Json } catch { $errors.Add("invalid-report-json") }
    if ($report) {
        if ([string]$report.schema -ne $script:FleetReportSchema -or [string]$Envelope.report.schema -ne $script:FleetReportSchema) {
            $errors.Add("unsupported-report-schema")
        }
        if ([string]$report.node.node_id -ne $nodeId) {
            $errors.Add("node-id-mismatch")
        }
        if ([bool]$report.destructive_actions_taken) {
            $errors.Add("destructive-report")
        }
        if ([bool]$report.privacy.absolute_paths_included -or [bool]$report.privacy.host_name_included -or [bool]$report.privacy.user_name_included) {
            $errors.Add("privacy-contract-failed")
        }
    }
    if ([bool]$Envelope.security.destructive_actions_authorized) {
        $errors.Add("destructive-authority-present")
    }
    if (-not [bool]$Envelope.security.receiver_must_authenticate_node) {
        $errors.Add("receiver-authentication-not-required")
    }

    [pscustomobject]@{
        schema = "elon.rust_cache.fleet_envelope_validation.v1"
        valid = $errors.Count -eq 0
        errors = @($errors)
        report = $report
    }
}

function Export-RustCacheFleetEnvelope {
    param(
        [Parameter(Mandatory)]$Envelope,
        [Parameter(Mandatory)][string]$CacheRoot,
        [string]$OutputPath
    )

    $validation = Test-RustCacheFleetEnvelope -Envelope $Envelope
    if (-not $validation.valid) {
        throw "Fleet envelope validation failed: $($validation.errors -join ', ')"
    }
    $root = [System.IO.Path]::GetFullPath($CacheRoot)
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        $queueRoot = Join-Path $root "reports\fleet\outbox"
        $name = "fleet-envelope-{0}-{1}.json" -f [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss"), [string]$Envelope.envelope_id
        $path = Join-Path $queueRoot $name
    } else {
        if (-not (Test-RustCacheAbsolutePath $OutputPath)) {
            throw "OutputPath must be absolute: $OutputPath"
        }
        $path = [System.IO.Path]::GetFullPath($OutputPath)
    }

    New-Item -ItemType Directory -Force -Path (Split-Path $path -Parent) | Out-Null
    $temporary = "$path.$PID.tmp"
    try {
        $payload = $Envelope | ConvertTo-Json -Depth 10
        [System.IO.File]::WriteAllText($temporary, $payload, $script:Utf8NoBom)
        Move-Item -LiteralPath $temporary -Destination $path -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }

    [pscustomobject]@{
        schema = "elon.rust_cache.fleet_stage.v1"
        envelope_path = $path
        envelope_sha256 = Get-RustCacheFileSha256 -Path $path
        report_sha256 = [string]$Envelope.report.content_sha256
        envelope = $Envelope
    }
}

Export-ModuleMember -Function New-RustCacheFleetEnvelope, Test-RustCacheFleetEnvelope, Export-RustCacheFleetEnvelope

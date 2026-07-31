function New-ElonReleaseReceipt {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [Parameter(Mandatory)] [string]$Kind,
        [Parameter(Mandatory)] [string]$SourceSha
    )

    if ($Kind -notmatch '^[a-z0-9_-]+$') { throw "Invalid release receipt kind: $Kind" }
    if ($SourceSha -notmatch '^[0-9a-f]{7,40}$') { throw "Invalid release source SHA: $SourceSha" }
    $root = Join-Path $RepoRoot '.ai-tmp\release-receipts'
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    $path = Join-Path $root "$Kind-$SourceSha.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $initial = [ordered]@{
            schema = 'elon.release.receipt.v1'
            kind = $Kind
            sourceSha = $SourceSha
            createdAt = [DateTimeOffset]::UtcNow.ToString('o')
            updatedAt = [DateTimeOffset]::UtcNow.ToString('o')
            stages = [ordered]@{}
        }
        Write-ElonReleaseReceiptFile -Path $path -Value $initial
    }
    [PSCustomObject]@{ Path = $path; Kind = $Kind; SourceSha = $SourceSha }
}

function Write-ElonReleaseReceiptFile {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] $Value
    )

    $temporary = "$Path.$PID.tmp"
    $json = $Value | ConvertTo-Json -Depth 12
    [System.IO.File]::WriteAllText($temporary, $json, (New-Object System.Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Read-ElonReleaseReceiptFile {
    param([Parameter(Mandatory)] [string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Set-ElonReleaseStageReceipt {
    param(
        [Parameter(Mandatory)] $Receipt,
        [Parameter(Mandatory)] [ValidatePattern('^[a-z0-9_-]+$')] [string]$Stage,
        [Parameter(Mandatory)] [ValidateSet('running', 'passed', 'failed', 'skipped')] [string]$Status,
        [double]$DurationSeconds = 0,
        [string]$Message = ''
    )

    $document = Read-ElonReleaseReceiptFile -Path $Receipt.Path
    if (-not $document) { throw "Release receipt disappeared: $($Receipt.Path)" }
    $record = [ordered]@{
        status = $Status
        recordedAt = [DateTimeOffset]::UtcNow.ToString('o')
        durationSeconds = [Math]::Round($DurationSeconds, 1)
        message = $Message
    }
    if (-not $document.stages) {
        $document | Add-Member -NotePropertyName stages -NotePropertyValue ([PSCustomObject]@{}) -Force
    }
    $document.stages | Add-Member -NotePropertyName $Stage -NotePropertyValue ([PSCustomObject]$record) -Force
    $document.updatedAt = [DateTimeOffset]::UtcNow.ToString('o')
    Write-ElonReleaseReceiptFile -Path $Receipt.Path -Value $document
    Write-Host "RELEASE_STAGE=$Stage status=$Status durationSeconds=$([Math]::Round($DurationSeconds, 1)) message=$Message"
}

function Test-ElonReleaseStagePassed {
    param(
        [Parameter(Mandatory)] $Receipt,
        [Parameter(Mandatory)] [string]$Stage
    )

    $document = Read-ElonReleaseReceiptFile -Path $Receipt.Path
    if (-not $document -or -not $document.stages) { return $false }
    $property = $document.stages.PSObject.Properties[$Stage]
    return $null -ne $property -and [string]$property.Value.status -eq 'passed'
}

function Invoke-ElonReleaseStage {
    param(
        [Parameter(Mandatory)] $Receipt,
        [Parameter(Mandatory)] [string]$Stage,
        [Parameter(Mandatory)] [scriptblock]$Action,
        [string]$SuccessMessage = 'completed'
    )

    Set-ElonReleaseStageReceipt -Receipt $Receipt -Stage $Stage -Status running
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        # A PowerShell-only action does not update LASTEXITCODE. Reset it so a
        # failed native command from an earlier stage cannot poison this one.
        $global:LASTEXITCODE = 0
        & $Action
        $exitCode = $LASTEXITCODE
        if ($null -ne $exitCode -and $exitCode -ne 0) {
            throw "$Stage failed with exit code $exitCode"
        }
        $watch.Stop()
        Set-ElonReleaseStageReceipt -Receipt $Receipt -Stage $Stage -Status passed `
            -DurationSeconds $watch.Elapsed.TotalSeconds -Message $SuccessMessage
    } catch {
        $watch.Stop()
        Set-ElonReleaseStageReceipt -Receipt $Receipt -Stage $Stage -Status failed `
            -DurationSeconds $watch.Elapsed.TotalSeconds -Message $_.Exception.Message
        throw
    }
}

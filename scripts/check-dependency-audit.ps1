param(
    [ValidateSet("Report", "Strict")]
    [string]$Mode = "Report",
    [string]$NpmProjectDir = "pc-frontend",
    [string]$RustManifestPath = "server\Cargo.toml",
    [switch]$SkipNpm,
    [switch]$SkipRust,
    [switch]$RequireRustAudit,
    [switch]$AllowStaleRustAdvisoryDb
)

$ErrorActionPreference = "Stop"

function Stop-DependencyAudit {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-DependencyAudit "Current directory is not inside a git repository."
    }
    return $root
}

function Invoke-ExternalCapture {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [string]$WorkingDirectory = ""
    )

    $stdoutFile = [System.IO.Path]::GetTempFileName()
    $stderrFile = [System.IO.Path]::GetTempFileName()
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $originalLocation = Get-Location
    try {
        if (-not [string]::IsNullOrWhiteSpace($WorkingDirectory)) {
            Set-Location $WorkingDirectory
        }
        & $FilePath @Arguments 1>$stdoutFile 2>$stderrFile
        $exitCode = $LASTEXITCODE
    } finally {
        Set-Location $originalLocation
        $ErrorActionPreference = $oldPreference
    }
    $output = Get-Content -LiteralPath $stdoutFile -Raw -ErrorAction SilentlyContinue
    $errorOutput = Get-Content -LiteralPath $stderrFile -Raw -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = if ($null -eq $output) { "" } else { $output.Trim() }
        Error = if ($null -eq $errorOutput) { "" } else { $errorOutput.Trim() }
    }
}

function Resolve-ApplicationPath {
    param([string]$Name)
    $command = @(Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($command.Count -eq 0 -or [string]::IsNullOrWhiteSpace([string]$command[0].Source)) {
        return ""
    }
    return [string]$command[0].Source
}

function ConvertFrom-JsonText {
    param(
        [string]$Text,
        [string]$Label
    )
    if ([string]::IsNullOrWhiteSpace($Text)) {
        Stop-DependencyAudit "$Label returned empty output."
    }
    try {
        return $Text | ConvertFrom-Json
    } catch {
        Stop-DependencyAudit "$Label returned non-JSON output: $($_.Exception.Message)"
    }
}

function Get-PropertyText {
    param(
        [AllowNull()][object]$Object,
        [string]$Name,
        [string]$Fallback = ""
    )
    if ($null -eq $Object) { return $Fallback }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) { return $Fallback }
    return [string]$property.Value
}

function Invoke-NpmDependencyAudit {
    param([string]$ProjectDir)

    $npm = Resolve-ApplicationPath "npm"
    if ([string]::IsNullOrWhiteSpace($npm)) {
        Write-Host "DEPENDENCY_AUDIT_NPM=skipped reason=npm-unavailable project=$ProjectDir"
        return 0
    }

    $packageLock = Join-Path $ProjectDir "package-lock.json"
    if (-not (Test-Path -LiteralPath $packageLock -PathType Leaf)) {
        Write-Host "DEPENDENCY_AUDIT_NPM=skipped reason=package-lock-missing project=$ProjectDir"
        return 0
    }

    $result = Invoke-ExternalCapture -FilePath $npm -Arguments @("audit", "--json") -WorkingDirectory $ProjectDir
    $json = ConvertFrom-JsonText -Text $result.Output -Label "npm audit"
    $counts = $json.metadata.vulnerabilities
    $total = [int](Get-PropertyText $counts "total" "0")
    $critical = [int](Get-PropertyText $counts "critical" "0")
    $high = [int](Get-PropertyText $counts "high" "0")
    $moderate = [int](Get-PropertyText $counts "moderate" "0")
    $low = [int](Get-PropertyText $counts "low" "0")
    $info = [int](Get-PropertyText $counts "info" "0")

    Write-Host "DEPENDENCY_AUDIT_NPM=reported project=$ProjectDir total=$total critical=$critical high=$high moderate=$moderate low=$low info=$info audit_exit=$($result.ExitCode)"

    $vulnerabilities = @()
    if ($null -ne $json.vulnerabilities) {
        $vulnerabilities = @($json.vulnerabilities.PSObject.Properties | ForEach-Object { $_.Value })
    }
    $rank = @{ critical = 4; high = 3; moderate = 2; low = 1; info = 0 }
    foreach ($vulnerability in ($vulnerabilities | Sort-Object -Property @{ Expression = { -1 * [int]$rank[[string]$_.severity] } }, name | Select-Object -First 10)) {
        $fix = Get-PropertyText $vulnerability "fixAvailable" ""
        if ($vulnerability.fixAvailable -is [bool]) {
            $fix = [string]$vulnerability.fixAvailable
        } elseif ($null -ne $vulnerability.fixAvailable) {
            $fixName = Get-PropertyText $vulnerability.fixAvailable "name" ""
            $fixVersion = Get-PropertyText $vulnerability.fixAvailable "version" ""
            $fixMajor = Get-PropertyText $vulnerability.fixAvailable "isSemVerMajor" ""
            $fix = "$fixName@$fixVersion major=$fixMajor"
        }
        Write-Host "DEPENDENCY_AUDIT_NPM_FINDING name=$($vulnerability.name) severity=$($vulnerability.severity) direct=$($vulnerability.isDirect) fix=$fix"
    }
    return $total
}

function Test-CargoAuditAvailable {
    $cargo = Resolve-ApplicationPath "cargo"
    if ([string]::IsNullOrWhiteSpace($cargo)) { return $false }
    $result = Invoke-ExternalCapture -FilePath $cargo -Arguments @("audit", "--version")
    return $result.ExitCode -eq 0
}

function Get-CargoMetadataPackageCount {
    param([string]$ManifestPath)

    $cargo = Resolve-ApplicationPath "cargo"
    if ([string]::IsNullOrWhiteSpace($cargo)) { return -1 }
    $result = Invoke-ExternalCapture -FilePath $cargo -Arguments @("metadata", "--manifest-path", $ManifestPath, "--format-version", "1", "--locked")
    if ($result.ExitCode -ne 0) { return -1 }
    $json = ConvertFrom-JsonText -Text $result.Output -Label "cargo metadata"
    return @($json.packages).Count
}

function Get-CargoAuditVulnerabilityCount {
    param([object]$AuditJson)

    $count = Get-PropertyText $AuditJson.vulnerabilities "count" ""
    if ($count -match '^\d+$') { return [int]$count }
    if ($null -ne $AuditJson.vulnerabilities.list) { return @($AuditJson.vulnerabilities.list).Count }
    if ($null -ne $AuditJson.vulnerabilities.found) { return @($AuditJson.vulnerabilities.found).Count }
    return 0
}

function Get-CargoAuditWarningCount {
    param([object]$AuditJson)

    if ($null -eq $AuditJson.warnings) { return 0 }
    $count = Get-PropertyText $AuditJson.warnings "count" ""
    if ($count -match '^\d+$') { return [int]$count }
    if ($null -ne $AuditJson.warnings.list) { return @($AuditJson.warnings.list).Count }

    $total = 0
    foreach ($property in @($AuditJson.warnings.PSObject.Properties)) {
        if ($null -ne $property.Value.list) {
            $total += @($property.Value.list).Count
        } elseif ($property.Value -is [array]) {
            $total += @($property.Value).Count
        }
    }
    return $total
}

function Invoke-CargoAuditJson {
    param(
        [string]$CargoPath,
        [string]$ManifestDir,
        [switch]$AllowStaleAdvisoryDb
    )

    $result = Invoke-ExternalCapture -FilePath $CargoPath -Arguments @("audit", "--json") -WorkingDirectory $ManifestDir
    if ($result.ExitCode -eq 0 -or -not [string]::IsNullOrWhiteSpace($result.Output) -or -not $AllowStaleAdvisoryDb) {
        return $result
    }

    Write-Host "DEPENDENCY_AUDIT_RUST_RETRY=stale-advisory-db reason=initial-audit-produced-no-json"
    return Invoke-ExternalCapture -FilePath $CargoPath -Arguments @("audit", "--json", "--no-fetch", "--stale") -WorkingDirectory $ManifestDir
}

function Invoke-RustDependencyAudit {
    param(
        [string]$ManifestPath,
        [switch]$RequireAuditTool,
        [switch]$AllowStaleAdvisoryDb
    )

    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
        Write-Host "DEPENDENCY_AUDIT_RUST=skipped reason=manifest-missing manifest=$ManifestPath"
        return 0
    }

    $packageCount = Get-CargoMetadataPackageCount -ManifestPath $ManifestPath
    if (-not (Test-CargoAuditAvailable)) {
        $message = "DEPENDENCY_AUDIT_RUST=skipped reason=cargo-audit-unavailable manifest=$ManifestPath packages=$packageCount hint='cargo install cargo-audit --locked'"
        if ($RequireAuditTool) {
            Write-Host ($message -replace "skipped", "failed")
            Stop-DependencyAudit "Rust dependency audit requires cargo-audit, but cargo audit --version did not succeed."
        }
        Write-Host $message
        return 0
    }

    $manifestDir = Split-Path $ManifestPath -Parent
    $cargo = Resolve-ApplicationPath "cargo"
    if ([string]::IsNullOrWhiteSpace($cargo)) {
        Write-Host "DEPENDENCY_AUDIT_RUST=skipped reason=cargo-unavailable manifest=$ManifestPath packages=$packageCount"
        return 0
    }
    $result = Invoke-CargoAuditJson -CargoPath $cargo -ManifestDir $manifestDir -AllowStaleAdvisoryDb:$AllowStaleAdvisoryDb
    if ($result.ExitCode -ne 0 -and [string]::IsNullOrWhiteSpace($result.Output)) {
        $errorText = if ([string]::IsNullOrWhiteSpace($result.Error)) { "empty stderr" } else { $result.Error }
        Stop-DependencyAudit "cargo audit failed before producing JSON: exit=$($result.ExitCode) stderr=$errorText"
    }
    $json = ConvertFrom-JsonText -Text $result.Output -Label "cargo audit"
    $count = Get-CargoAuditVulnerabilityCount -AuditJson $json
    $warningCount = Get-CargoAuditWarningCount -AuditJson $json
    Write-Host "DEPENDENCY_AUDIT_RUST=reported manifest=$ManifestPath vulnerabilities=$count warnings=$warningCount packages=$packageCount audit_exit=$($result.ExitCode)"

    if ($null -ne $json.vulnerabilities.list) {
        foreach ($item in @($json.vulnerabilities.list | Select-Object -First 10)) {
            $advisoryId = Get-PropertyText $item.advisory "id" ""
            $packageName = Get-PropertyText $item.package "name" ""
            $title = Get-PropertyText $item.advisory "title" ""
            Write-Host "DEPENDENCY_AUDIT_RUST_FINDING id=$advisoryId package=$packageName title=$title"
        }
    }
    if ($null -ne $json.warnings) {
        foreach ($warningGroup in @($json.warnings.PSObject.Properties | Select-Object -First 10)) {
            $items = @()
            if ($null -ne $warningGroup.Value.list) {
                $items = @($warningGroup.Value.list)
            } elseif ($warningGroup.Value -is [array]) {
                $items = @($warningGroup.Value)
            }
            foreach ($item in ($items | Select-Object -First 10)) {
                $advisoryId = Get-PropertyText $item.advisory "id" ""
                $packageName = Get-PropertyText $item.package "name" ""
                $title = Get-PropertyText $item.advisory "title" ""
                Write-Host "DEPENDENCY_AUDIT_RUST_WARNING kind=$($warningGroup.Name) id=$advisoryId package=$packageName title=$title"
            }
        }
    }
    return $count
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$npmProjectFullPath = Join-Path $repoRoot $NpmProjectDir
$rustManifestFullPath = Join-Path $repoRoot $RustManifestPath

Write-Host "DEPENDENCY_AUDIT_MODE=$Mode"
$npmIssues = 0
$rustIssues = 0

if (-not $SkipNpm) {
    $npmIssues = Invoke-NpmDependencyAudit -ProjectDir $npmProjectFullPath
}
if (-not $SkipRust) {
    $rustIssues = Invoke-RustDependencyAudit -ManifestPath $rustManifestFullPath -RequireAuditTool:$RequireRustAudit -AllowStaleAdvisoryDb:$AllowStaleRustAdvisoryDb
}

if ($Mode -eq "Strict" -and (($npmIssues + $rustIssues) -gt 0)) {
    Stop-DependencyAudit "Dependency audit strict mode failed: npm=$npmIssues rust=$rustIssues"
}

Write-Host "DEPENDENCY_AUDIT=passed mode=$Mode npm=$npmIssues rust=$rustIssues"

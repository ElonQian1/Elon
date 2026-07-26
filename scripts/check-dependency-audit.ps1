param(
    [ValidateSet("Report", "Strict")]
    [string]$Mode = "Report",
    [string]$NpmProjectDir = "pc-frontend",
    [string]$RustManifestPath = "server\Cargo.toml",
    [string]$NpmExceptionsPath = ".github\dependency-audit-exceptions.json",
    [string]$RustExceptionsPath = ".github\dependency-audit-exceptions.json",
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

function Get-NpmAuditExceptions {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }

    $config = ConvertFrom-JsonText -Text (Get-Content -LiteralPath $Path -Raw) -Label "npm audit exceptions"
    if ((Get-PropertyText $config "version") -ne "1") {
        Stop-DependencyAudit "npm audit exceptions must declare version=1: $Path"
    }
    if ($null -eq $config.npm) {
        return @()
    }

    $today = (Get-Date).Date
    foreach ($entry in @($config.npm)) {
        $name = Get-PropertyText $entry "name"
        $packageVersion = Get-PropertyText $entry "packageVersion"
        $source = Get-PropertyText $entry "source"
        $expiresOn = Get-PropertyText $entry "expiresOn"
        $reason = Get-PropertyText $entry "reason"
        if ([string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($packageVersion) -or $source -notmatch '^\d+$' -or [string]::IsNullOrWhiteSpace($expiresOn) -or [string]::IsNullOrWhiteSpace($reason)) {
            Stop-DependencyAudit "npm audit exception is missing name, packageVersion, numeric source, expiresOn, or reason: $Path"
        }
        try {
            $expiresAt = [datetime]::ParseExact($expiresOn, "yyyy-MM-dd", [Globalization.CultureInfo]::InvariantCulture)
        } catch {
            Stop-DependencyAudit "npm audit exception has invalid expiresOn '$expiresOn' (expected yyyy-MM-dd): $Path"
        }
        if ($expiresAt.Date -lt $today) {
            Stop-DependencyAudit "npm audit exception expired on $expiresOn for $name@$packageVersion source=${source}: $Path"
        }
    }
    return @($config.npm)
}

function Get-RustAuditExceptions {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }

    $config = ConvertFrom-JsonText -Text (Get-Content -LiteralPath $Path -Raw) -Label "RustSec audit exceptions"
    if ((Get-PropertyText $config "version") -ne "1") {
        Stop-DependencyAudit "RustSec audit exceptions must declare version=1: $Path"
    }
    if ($null -eq $config.rust) {
        return @()
    }

    $today = (Get-Date).Date
    $keys = New-Object System.Collections.Generic.HashSet[string]
    foreach ($entry in @($config.rust)) {
        $id = Get-PropertyText $entry "id"
        $name = Get-PropertyText $entry "name"
        $packageVersion = Get-PropertyText $entry "packageVersion"
        $expiresOn = Get-PropertyText $entry "expiresOn"
        $reason = Get-PropertyText $entry "reason"
        if ($id -notmatch '^RUSTSEC-\d{4}-\d{4}$' -or [string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($packageVersion) -or [string]::IsNullOrWhiteSpace($expiresOn) -or [string]::IsNullOrWhiteSpace($reason)) {
            Stop-DependencyAudit "RustSec audit exception is missing a valid id, name, packageVersion, expiresOn, or reason: $Path"
        }
        try {
            $expiresAt = [datetime]::ParseExact($expiresOn, "yyyy-MM-dd", [Globalization.CultureInfo]::InvariantCulture)
        } catch {
            Stop-DependencyAudit "RustSec audit exception has invalid expiresOn '$expiresOn' (expected yyyy-MM-dd): $Path"
        }
        if ($expiresAt.Date -lt $today) {
            Stop-DependencyAudit "RustSec audit exception expired on $expiresOn for $id ${name}@${packageVersion}: $Path"
        }
        $key = "$id|$name|$packageVersion"
        if (-not $keys.Add($key)) {
            Stop-DependencyAudit "RustSec audit exception is duplicated for $id ${name}@${packageVersion}: $Path"
        }
    }
    return @($config.rust)
}

function Get-NpmPackageLockVersions {
    param([string]$ProjectDir)

    $packageLock = Join-Path $ProjectDir "package-lock.json"
    $node = Resolve-ApplicationPath "node"
    if ([string]::IsNullOrWhiteSpace($node)) {
        Stop-DependencyAudit "node is required to parse package-lock.json for npm audit exceptions."
    }
    # npm lockfiles can contain an empty dependency key, which Windows
    # PowerShell 5.1's JSON object conversion cannot represent. Node parses
    # npm's own JSON faithfully and returns the small version map we need.
    $nodeScript = @'
const fs = require('fs');
const lock = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
const versions = {};
for (const [path, entry] of Object.entries(lock.packages || {})) {
  if (path.startsWith('node_modules/') && typeof entry.version === 'string') {
    versions[path.slice('node_modules/'.length)] = entry.version;
  }
}
process.stdout.write(JSON.stringify(versions));
'@
    $result = Invoke-ExternalCapture -FilePath $node -Arguments @("-e", $nodeScript, $packageLock)
    if ($result.ExitCode -ne 0) {
        Stop-DependencyAudit "package-lock version extraction failed: $($result.Error)"
    }
    $lockVersions = ConvertFrom-JsonText -Text $result.Output -Label "package-lock version map"
    $versions = @{}
    foreach ($property in @($lockVersions.PSObject.Properties)) {
        $version = [string]$property.Value
        if ([string]::IsNullOrWhiteSpace($version)) { continue }
        $versions[$property.Name] = $version
    }
    return $versions
}

function Get-NpmAuditSourceIds {
    param([object]$Vulnerability)

    $sources = New-Object System.Collections.Generic.List[string]
    foreach ($via in @($Vulnerability.via)) {
        if ($via -is [string]) { continue }
        $source = Get-PropertyText $via "source"
        if ($source -match '^\d+$' -and -not $sources.Contains($source)) {
            $sources.Add($source)
        }
    }
    return $sources.ToArray()
}

function Get-NpmAuditViaPackages {
    param([object]$Vulnerability)

    $packages = New-Object System.Collections.Generic.List[string]
    foreach ($via in @($Vulnerability.via)) {
        if ($via -is [string] -and -not [string]::IsNullOrWhiteSpace($via) -and -not $packages.Contains($via)) {
            $packages.Add($via)
        }
    }
    return $packages.ToArray()
}

function Test-NpmAuditException {
    param(
        [object]$Vulnerability,
        [hashtable]$VulnerabilitiesByName,
        [hashtable]$PackageVersions,
        [object[]]$Exceptions,
        [string[]]$Visited = @()
    )

    $name = Get-PropertyText $Vulnerability "name"
    if ([string]::IsNullOrWhiteSpace($name) -or $Visited -contains $name) {
        return [pscustomobject]@{ Excepted = $false; Entries = @() }
    }
    $version = if ($PackageVersions.ContainsKey($name)) { [string]$PackageVersions[$name] } else { "" }
    $nextVisited = @($Visited + $name)
    $sources = @(Get-NpmAuditSourceIds -Vulnerability $Vulnerability)

    if ($sources.Count -gt 0) {
        $matchedEntries = New-Object System.Collections.Generic.List[object]
        foreach ($source in $sources) {
            $match = @($Exceptions | Where-Object {
                (Get-PropertyText $_ "name") -eq $name -and
                (Get-PropertyText $_ "packageVersion") -eq $version -and
                (Get-PropertyText $_ "source") -eq $source
            })
            if ($match.Count -ne 1) {
                return [pscustomobject]@{ Excepted = $false; Entries = @() }
            }
            $matchedEntries.Add($match[0])
        }
        return [pscustomobject]@{ Excepted = $true; Entries = $matchedEntries.ToArray() }
    }

    $viaPackages = @(Get-NpmAuditViaPackages -Vulnerability $Vulnerability)
    if ($viaPackages.Count -eq 0) {
        return [pscustomobject]@{ Excepted = $false; Entries = @() }
    }

    $inheritedEntries = New-Object System.Collections.Generic.List[object]
    foreach ($viaPackage in $viaPackages) {
        if (-not $VulnerabilitiesByName.ContainsKey($viaPackage)) {
            return [pscustomobject]@{ Excepted = $false; Entries = @() }
        }
        $parent = Test-NpmAuditException -Vulnerability $VulnerabilitiesByName[$viaPackage] -VulnerabilitiesByName $VulnerabilitiesByName -PackageVersions $PackageVersions -Exceptions $Exceptions -Visited $nextVisited
        if (-not $parent.Excepted) {
            return [pscustomobject]@{ Excepted = $false; Entries = @() }
        }
        foreach ($entry in @($parent.Entries)) {
            $inheritedEntries.Add($entry)
        }
    }
    return [pscustomobject]@{ Excepted = $true; Entries = $inheritedEntries.ToArray() }
}

function Invoke-NpmDependencyAudit {
    param(
        [string]$ProjectDir,
        [string]$ExceptionsPath
    )

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

    $vulnerabilities = @()
    if ($null -ne $json.vulnerabilities) {
        $vulnerabilities = @($json.vulnerabilities.PSObject.Properties | ForEach-Object { $_.Value })
    }
    $exceptions = @(Get-NpmAuditExceptions -Path $ExceptionsPath)
    $packageVersions = Get-NpmPackageLockVersions -ProjectDir $ProjectDir
    $vulnerabilitiesByName = @{}
    foreach ($vulnerability in $vulnerabilities) {
        $vulnerabilitiesByName[(Get-PropertyText $vulnerability "name")] = $vulnerability
    }

    $blocking = New-Object System.Collections.Generic.List[object]
    $exceptionLabels = New-Object System.Collections.Generic.HashSet[string]
    foreach ($vulnerability in $vulnerabilities) {
        $exception = Test-NpmAuditException -Vulnerability $vulnerability -VulnerabilitiesByName $vulnerabilitiesByName -PackageVersions $packageVersions -Exceptions $exceptions
        if (-not $exception.Excepted) {
            $blocking.Add($vulnerability)
            continue
        }
        foreach ($entry in @($exception.Entries)) {
            $label = "$(Get-PropertyText $entry 'name')@$(Get-PropertyText $entry 'packageVersion') source=$(Get-PropertyText $entry 'source') expires=$(Get-PropertyText $entry 'expiresOn')"
            if ($exceptionLabels.Add($label)) {
                Write-Host "DEPENDENCY_AUDIT_NPM_EXCEPTION=$label"
            }
        }
    }

    foreach ($entry in $exceptions) {
        $label = "$(Get-PropertyText $entry 'name')@$(Get-PropertyText $entry 'packageVersion') source=$(Get-PropertyText $entry 'source') expires=$(Get-PropertyText $entry 'expiresOn')"
        if (-not $exceptionLabels.Contains($label)) {
            Stop-DependencyAudit "npm audit exception is not matched by the current report and must be removed or corrected: $label"
        }
    }

    Write-Host "DEPENDENCY_AUDIT_NPM=reported project=$ProjectDir total=$total critical=$critical high=$high moderate=$moderate low=$low info=$info blocking=$($blocking.Count) exceptions=$($exceptionLabels.Count) audit_exit=$($result.ExitCode)"
    $rank = @{ critical = 4; high = 3; moderate = 2; low = 1; info = 0 }
    foreach ($vulnerability in ($blocking | Sort-Object -Property @{ Expression = { -1 * [int]$rank[[string]$_.severity] } }, name | Select-Object -First 10)) {
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
    return $blocking.Count
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

function Get-RustAuditExceptionKey {
    param([object]$Entry)
    return "$(Get-PropertyText $Entry 'id')|$(Get-PropertyText $Entry 'name')|$(Get-PropertyText $Entry 'packageVersion')"
}

function Test-RustAuditException {
    param(
        [object]$Vulnerability,
        [object[]]$Exceptions
    )

    $id = Get-PropertyText $Vulnerability.advisory "id"
    $name = Get-PropertyText $Vulnerability.package "name"
    $packageVersion = Get-PropertyText $Vulnerability.package "version"
    $matches = @($Exceptions | Where-Object {
        (Get-PropertyText $_ "id") -eq $id -and
        (Get-PropertyText $_ "name") -eq $name -and
        (Get-PropertyText $_ "packageVersion") -eq $packageVersion
    })
    if ($matches.Count -ne 1) {
        return $null
    }
    return $matches[0]
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
        [string]$ExceptionsPath,
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
    $vulnerabilities = @()
    if ($null -ne $json.vulnerabilities.list) {
        $vulnerabilities = @($json.vulnerabilities.list)
    }
    if ($count -ne $vulnerabilities.Count) {
        Stop-DependencyAudit "cargo audit reported vulnerabilities=$count but did not provide a matching vulnerability list for exception evaluation."
    }
    $exceptions = @(Get-RustAuditExceptions -Path $ExceptionsPath)
    $blocking = New-Object System.Collections.Generic.List[object]
    $matchedExceptionKeys = New-Object System.Collections.Generic.HashSet[string]
    foreach ($item in $vulnerabilities) {
        $exception = Test-RustAuditException -Vulnerability $item -Exceptions $exceptions
        if ($null -eq $exception) {
            $blocking.Add($item)
            continue
        }
        $key = Get-RustAuditExceptionKey -Entry $exception
        if ($matchedExceptionKeys.Add($key)) {
            Write-Host "DEPENDENCY_AUDIT_RUST_EXCEPTION=id=$(Get-PropertyText $exception 'id') package=$(Get-PropertyText $exception 'name')@$(Get-PropertyText $exception 'packageVersion') expires=$(Get-PropertyText $exception 'expiresOn')"
        }
    }
    foreach ($exception in $exceptions) {
        $key = Get-RustAuditExceptionKey -Entry $exception
        if (-not $matchedExceptionKeys.Contains($key)) {
            Stop-DependencyAudit "RustSec audit exception is not matched by the current report and must be removed or corrected: $key"
        }
    }
    Write-Host "DEPENDENCY_AUDIT_RUST=reported manifest=$ManifestPath vulnerabilities=$count blocking=$($blocking.Count) exceptions=$($matchedExceptionKeys.Count) warnings=$warningCount packages=$packageCount audit_exit=$($result.ExitCode)"

    foreach ($item in @($blocking | Select-Object -First 10)) {
            $advisoryId = Get-PropertyText $item.advisory "id" ""
            $packageName = Get-PropertyText $item.package "name" ""
            $title = Get-PropertyText $item.advisory "title" ""
            Write-Host "DEPENDENCY_AUDIT_RUST_FINDING id=$advisoryId package=$packageName title=$title"
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
    return $blocking.Count
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$npmProjectFullPath = Join-Path $repoRoot $NpmProjectDir
$rustManifestFullPath = Join-Path $repoRoot $RustManifestPath
$npmExceptionsFullPath = Join-Path $repoRoot $NpmExceptionsPath
$rustExceptionsFullPath = Join-Path $repoRoot $RustExceptionsPath

Write-Host "DEPENDENCY_AUDIT_MODE=$Mode"
$npmIssues = 0
$rustIssues = 0

if (-not $SkipNpm) {
    $npmIssues = Invoke-NpmDependencyAudit -ProjectDir $npmProjectFullPath -ExceptionsPath $npmExceptionsFullPath
}
if (-not $SkipRust) {
    $rustIssues = Invoke-RustDependencyAudit -ManifestPath $rustManifestFullPath -ExceptionsPath $rustExceptionsFullPath -RequireAuditTool:$RequireRustAudit -AllowStaleAdvisoryDb:$AllowStaleRustAdvisoryDb
}

if ($Mode -eq "Strict" -and (($npmIssues + $rustIssues) -gt 0)) {
    Stop-DependencyAudit "Dependency audit strict mode failed: npm=$npmIssues rust=$rustIssues"
}

Write-Host "DEPENDENCY_AUDIT=passed mode=$Mode npm=$npmIssues rust=$rustIssues"
exit 0

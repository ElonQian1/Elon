$ErrorActionPreference = 'Stop'

function Assert-StitchWorkflow {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        Write-Error "STITCH_WORKFLOW_TEST_FAILED=$Message"
        exit 1
    }
}

function New-TestPngHeader {
    param([int]$Width = 390, [int]$Height = 791)
    $bytes = New-Object byte[] 24
    [byte[]]$signature = 137, 80, 78, 71, 13, 10, 26, 10
    [Array]::Copy($signature, 0, $bytes, 0, $signature.Length)
    $bytes[11] = 13
    [byte[]]$ihdr = 73, 72, 68, 82
    [Array]::Copy($ihdr, 0, $bytes, 12, $ihdr.Length)
    $widthBytes = [BitConverter]::GetBytes([Net.IPAddress]::HostToNetworkOrder($Width))
    $heightBytes = [BitConverter]::GetBytes([Net.IPAddress]::HostToNetworkOrder($Height))
    [Array]::Copy($widthBytes, 0, $bytes, 16, 4)
    [Array]::Copy($heightBytes, 0, $bytes, 20, 4)
    return $bytes
}

function New-StitchFixture {
    param(
        [string]$Name,
        [switch]$Code,
        [switch]$Design,
        [switch]$Screen,
        [switch]$InvalidScreen,
        [switch]$DuplicateScreen
    )
    $source = Join-Path $testRoot "$Name-source"
    [IO.Directory]::CreateDirectory((Join-Path $source 'nested')) | Out-Null
    if ($Code) {
        [IO.File]::WriteAllText((Join-Path $source 'nested\code.html'), '<main>fixture</main>', $utf8NoBom)
    }
    if ($Design) {
        [IO.File]::WriteAllText((Join-Path $source 'nested\DESIGN.md'), '# Fixture', $utf8NoBom)
    }
    if ($Screen -or $InvalidScreen -or $DuplicateScreen) {
        $screenBytes = if ($InvalidScreen) { [byte[]](1..24) } else { New-TestPngHeader }
        [IO.File]::WriteAllBytes((Join-Path $source 'nested\screen.png'), $screenBytes)
    }
    if ($DuplicateScreen) {
        [IO.Directory]::CreateDirectory((Join-Path $source 'duplicate')) | Out-Null
        [IO.File]::WriteAllBytes((Join-Path $source 'duplicate\screen.png'), (New-TestPngHeader))
    }
    $zip = Join-Path $testRoot "$Name.zip"
    [IO.Compression.ZipFile]::CreateFromDirectory($source, $zip)
    return $zip
}

function Invoke-StitchInspector {
    param([string]$Zip, [string[]]$ExtraArguments = @())
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $inspector -ZipPath $Zip @ExtraArguments 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = [string]::Join([Environment]::NewLine, [string[]]$output)
    }
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
Assert-StitchWorkflow ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($repoRoot)) 'repository_root_unavailable'
$inspector = Join-Path $repoRoot 'scripts\inspect-stitch-export.ps1'
$utf8NoBom = New-Object Text.UTF8Encoding($false)
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("elon-stitch-workflow-{0}" -f [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($testRoot) | Out-Null
Add-Type -AssemblyName System.IO.Compression.FileSystem

try {
    $fullZip = New-StitchFixture -Name 'full' -Code -Design -Screen
    $receiptPath = Join-Path $testRoot 'receipt\inspection.json'
    $full = Invoke-StitchInspector -Zip $fullZip -ExtraArguments @('-RequireFull', '-OutputPath', $receiptPath)
    Assert-StitchWorkflow ($full.ExitCode -eq 0) "full_exit_$($full.ExitCode)"
    $fullReceipt = [IO.File]::ReadAllText($receiptPath, [Text.Encoding]::UTF8) | ConvertFrom-Json
    Assert-StitchWorkflow ($fullReceipt.schema -eq 'elon.stitch_export_inspection.v1') 'schema'
    Assert-StitchWorkflow ($fullReceipt.quality -eq 'FULL') 'full_quality'
    Assert-StitchWorkflow ($fullReceipt.designViewport.width -eq 390 -and $fullReceipt.designViewport.height -eq 791) 'viewport'
    Assert-StitchWorkflow ($fullReceipt.claimPolicy.exportEvidenceGate -eq 'PASSED') 'full_export_gate'
    Assert-StitchWorkflow (-not $fullReceipt.claimPolicy.oneToOneClaimFromExportAlone) 'export_must_not_claim_visual_acceptance'
    $receiptBytes = [IO.File]::ReadAllBytes($receiptPath)
    Assert-StitchWorkflow (-not ($receiptBytes.Length -ge 3 -and $receiptBytes[0] -eq 239 -and $receiptBytes[1] -eq 187 -and $receiptBytes[2] -eq 191)) 'receipt_utf8_bom'

    $partialZip = New-StitchFixture -Name 'partial' -Screen
    $partial = Invoke-StitchInspector -Zip $partialZip
    Assert-StitchWorkflow ($partial.ExitCode -eq 0) "partial_advisory_exit_$($partial.ExitCode)"
    Assert-StitchWorkflow (($partial.Output | ConvertFrom-Json).quality -eq 'PARTIAL') 'partial_quality'
    $partialRequired = Invoke-StitchInspector -Zip $partialZip -ExtraArguments @('-RequireFull')
    Assert-StitchWorkflow ($partialRequired.ExitCode -eq 3) "partial_required_exit_$($partialRequired.ExitCode)"

    $insufficientZip = New-StitchFixture -Name 'insufficient' -Code
    $insufficient = Invoke-StitchInspector -Zip $insufficientZip
    Assert-StitchWorkflow ($insufficient.ExitCode -eq 2) "insufficient_exit_$($insufficient.ExitCode)"
    Assert-StitchWorkflow (($insufficient.Output | ConvertFrom-Json).quality -eq 'INSUFFICIENT') 'insufficient_quality'

    $invalidZip = New-StitchFixture -Name 'invalid-screen' -Code -Design -InvalidScreen
    $invalid = Invoke-StitchInspector -Zip $invalidZip
    Assert-StitchWorkflow ($invalid.ExitCode -eq 1 -and $invalid.Output.Contains('STITCH_EXPORT_INVALID=screen_not_png')) 'invalid_screen'

    $duplicateZip = New-StitchFixture -Name 'duplicate-screen' -Code -Design -DuplicateScreen
    $duplicate = Invoke-StitchInspector -Zip $duplicateZip
    Assert-StitchWorkflow ($duplicate.ExitCode -eq 1 -and $duplicate.Output.Contains('STITCH_EXPORT_INVALID=duplicate_required_file:screen.png')) 'duplicate_screen'

    $agents = [IO.File]::ReadAllText((Join-Path $repoRoot 'AGENTS.md'), [Text.Encoding]::UTF8)
    $syncRules = [IO.File]::ReadAllText((Join-Path $repoRoot '.github\instructions\apk-web-ui-sync.instructions.md'), [Text.Encoding]::UTF8)
    $contract = [IO.File]::ReadAllText((Join-Path $repoRoot 'docs\stitch-design-import.md'), [Text.Encoding]::UTF8)
    Assert-StitchWorkflow ($agents.Contains('Stitch/Figma')) 'agents_route'
    Assert-StitchWorkflow ($syncRules.Contains('stitch-export-inspection.json') -and $syncRules.Contains('-RequireFull')) 'sync_gate'
    Assert-StitchWorkflow ($contract.Contains('exit 3') -and $contract.Contains('oneToOneClaimFromExportAlone')) 'documented_exit_contract'

    Write-Output 'STITCH_WORKFLOW_TEST=passed'
    Write-Output 'STITCH_WORKFLOW_CASES=full,partial,insufficient,invalid_png,duplicate_required_file,project_routing'
} finally {
    $resolvedTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $resolvedTestRoot = [IO.Path]::GetFullPath($testRoot)
    if ($resolvedTestRoot.StartsWith($resolvedTemp, [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolvedTestRoot)) {
        Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
    }
}

param(
    [string[]]$ContractTest = @(),
    [string]$NoContractReason = "",
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [string]$Label,
        [scriptblock]$Action
    )
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
    $watch.Stop()
    Write-Host "$($Label.ToUpperInvariant().Replace(' ', '_'))=passed durationSeconds=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
}

function Get-ChangedPaths {
    param([string]$Root)
    $paths = New-Object System.Collections.Generic.List[string]
    $commands = @(
        @("diff", "--name-only", "--diff-filter=ACMR", "origin/main...HEAD"),
        @("diff", "--name-only", "--diff-filter=ACMR"),
        @("diff", "--cached", "--name-only", "--diff-filter=ACMR"),
        @("ls-files", "--others", "--exclude-standard")
    )
    foreach ($arguments in $commands) {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & git -C $Root @arguments 2>$null
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        if ($LASTEXITCODE -eq 0) {
            foreach ($item in $output) {
                if (-not [string]::IsNullOrWhiteSpace($item)) {
                    $paths.Add(($item.Trim() -replace "\\", "/"))
                }
            }
        }
    }
    return @($paths | Sort-Object -Unique)
}

if ($ContractTest.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($NoContractReason)) {
    throw "Use either -ContractTest or -NoContractReason, not both."
}
if ($ContractTest.Count -eq 0 -and [string]::IsNullOrWhiteSpace($NoContractReason)) {
    throw "Specify a focused -ContractTest or explain why it is not applicable with -NoContractReason."
}

$repoRoot = (& git rev-parse --show-toplevel 2>&1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this script inside the repository."
}
$repoRoot = $repoRoot.Trim()
$androidRoot = Join-Path $repoRoot "android"
$gradle = Join-Path $androidRoot "gradlew.bat"
$pwaCheck = Join-Path $repoRoot "scripts\check-mobile-pwa-source.js"
$changedPaths = @(Get-ChangedPaths -Root $repoRoot)
$androidUiChanged = @($changedPaths | Where-Object {
    $_ -like "android/app/src/main/*"
}).Count -gt 0
$mobilePwaChanged = $changedPaths -contains "server/src/assets/web_page.html"

Write-Host "FAST_LANE_ANDROID_CHANGED=$($androidUiChanged.ToString().ToLowerInvariant())"
Write-Host "FAST_LANE_MOBILE_PWA_CHANGED=$($mobilePwaChanged.ToString().ToLowerInvariant())"

if ($androidUiChanged -and -not $mobilePwaChanged) {
    throw "Android UI changed without server/src/assets/web_page.html. Keep Android and mobile PWA in the same commit."
}
if ($PlanOnly) {
    Write-Host "FAST_LANE_PLAN=contract_then_parallel_android_and_mobile_pwa"
    Write-Host "FAST_LANE_RENDERER=skipped"
    exit 0
}

Invoke-Checked -Label "git diff check" -Action {
    & git -C $repoRoot diff --check
}

if ($ContractTest.Count -gt 0) {
    $contractArgs = @(":app:testDebugUnitTest", "--console=plain")
    foreach ($testName in $ContractTest) {
        $contractArgs += @("--tests", $testName)
    }
    Invoke-Checked -Label "contract test" -Action {
        Push-Location -LiteralPath $androidRoot
        try {
            & $gradle @contractArgs
        } finally {
            Pop-Location
        }
    }
} else {
    Write-Host "CONTRACT_TEST=not_applicable reason=$NoContractReason"
}

$tmpRoot = Join-Path $repoRoot ".ai-tmp\app-ui-fast-lane"
New-Item -ItemType Directory -Path $tmpRoot -Force | Out-Null
$androidOut = Join-Path $tmpRoot "android.out.log"
$androidErr = Join-Path $tmpRoot "android.err.log"
$pwaOut = Join-Path $tmpRoot "pwa.out.log"
$pwaErr = Join-Path $tmpRoot "pwa.err.log"
Remove-Item -LiteralPath $androidOut, $androidErr, $pwaOut, $pwaErr -Force -ErrorAction SilentlyContinue

$watch = [System.Diagnostics.Stopwatch]::StartNew()
$androidProcess = Start-Process -FilePath $gradle `
    -ArgumentList @(":app:assembleDebug", "--console=plain") `
    -WorkingDirectory $androidRoot -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput $androidOut -RedirectStandardError $androidErr
$pwaProcess = Start-Process -FilePath "node.exe" `
    -ArgumentList @($pwaCheck) -WorkingDirectory $repoRoot -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput $pwaOut -RedirectStandardError $pwaErr

$androidProcess.WaitForExit()
$pwaProcess.WaitForExit()
$watch.Stop()

$androidLog = if (Test-Path $androidOut) { Get-Content -Raw -LiteralPath $androidOut } else { "" }
$androidError = if (Test-Path $androidErr) { Get-Content -Raw -LiteralPath $androidErr } else { "" }
$pwaLog = if (Test-Path $pwaOut) { Get-Content -Raw -LiteralPath $pwaOut } else { "" }
$pwaError = if (Test-Path $pwaErr) { Get-Content -Raw -LiteralPath $pwaErr } else { "" }
if ($androidProcess.ExitCode -ne 0) {
    throw "Android assembleDebug failed:`n$androidLog`n$androidError"
}
if ($pwaProcess.ExitCode -ne 0) {
    throw "Mobile PWA source validation failed:`n$pwaLog`n$pwaError"
}

Write-Host $pwaLog.Trim()
Write-Host "ANDROID_ASSEMBLE_DEBUG=passed"
Write-Host "FAST_LANE_PARALLEL_DURATION_SECONDS=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
Write-Host "FAST_LANE_RENDERER=skipped"
Write-Host "FAST_LANE_RESULT=passed"

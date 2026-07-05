param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 5173,
    [switch]$Foreground,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Resolve-NpmCmd {
    if (-not [string]::IsNullOrWhiteSpace($env:NPM_CMD)) {
        if (-not (Test-Path -LiteralPath $env:NPM_CMD -PathType Leaf)) {
            throw "NPM_CMD points to a missing file: $env:NPM_CMD"
        }
        $ext = [System.IO.Path]::GetExtension($env:NPM_CMD)
        if ($ext -in @(".cmd", ".exe", ".bat")) {
            return (Resolve-Path -LiteralPath $env:NPM_CMD).Path
        }
        throw "NPM_CMD must point to npm.cmd/npm.exe/npm.bat on Windows, got: $env:NPM_CMD"
    }

    $npmCmd = Get-Command "npm.cmd" -ErrorAction SilentlyContinue
    if ($npmCmd -and $npmCmd.Source -and (Test-Path -LiteralPath $npmCmd.Source -PathType Leaf)) {
        return $npmCmd.Source
    }

    $npm = Get-Command "npm" -ErrorAction SilentlyContinue
    if ($npm -and $npm.Source) {
        $sourceExt = [System.IO.Path]::GetExtension($npm.Source)
        if ($sourceExt -in @(".cmd", ".exe", ".bat")) {
            return $npm.Source
        }

        $siblingCmd = Join-Path (Split-Path -Parent $npm.Source) "npm.cmd"
        if (Test-Path -LiteralPath $siblingCmd -PathType Leaf) {
            return $siblingCmd
        }

        if ($sourceExt -eq ".ps1") {
            throw "PowerShell resolved npm to $($npm.Source). Use npm.cmd for Start-Process to avoid opening npm.ps1 with the Windows file association."
        }
    }

    $node = Get-Command "node.exe" -ErrorAction SilentlyContinue
    if ($node -and $node.Source) {
        $candidate = Join-Path (Split-Path -Parent $node.Source) "npm.cmd"
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    throw "npm.cmd was not found. Install Node.js or set NPM_CMD to the full path of npm.cmd."
}

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if (-not $repoRoot) {
    $repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
}
$repoRoot = "$repoRoot".Trim()
$pcFrontendDir = Join-Path $repoRoot "pc-frontend"
if (-not (Test-Path -LiteralPath (Join-Path $pcFrontendDir "package.json") -PathType Leaf)) {
    throw "pc-frontend/package.json not found under $repoRoot"
}

$npmPath = Resolve-NpmCmd
$arguments = @("run", "dev", "--", "--host", $HostName, "--port", "$Port")

$nodeModulesMissing = -not (Test-Path -LiteralPath (Join-Path $pcFrontendDir "node_modules") -PathType Container)
if ($nodeModulesMissing) {
    if ($DryRun) {
        Write-Host "[pc-frontend-dev] dry run: node_modules is missing; would run npm ci first" -ForegroundColor Yellow
    } else {
        Write-Host "[pc-frontend-dev] node_modules is missing; running npm ci first..." -ForegroundColor Yellow
        Push-Location $pcFrontendDir
        try {
            & $npmPath ci
            if ($LASTEXITCODE -ne 0) {
                throw "npm ci failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    }
}

Write-Host "[pc-frontend-dev] npm executable: $npmPath" -ForegroundColor Gray
Write-Host "[pc-frontend-dev] working directory: $pcFrontendDir" -ForegroundColor Gray
Write-Host "[pc-frontend-dev] url: http://${HostName}:$Port/pc" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host "[pc-frontend-dev] dry run: $npmPath $($arguments -join ' ')" -ForegroundColor Cyan
    exit 0
}

if ($Foreground) {
    Push-Location $pcFrontendDir
    try {
        & $npmPath @arguments
        exit $LASTEXITCODE
    } finally {
        Pop-Location
    }
}

# Start-Process must receive npm.cmd. Passing bare npm can resolve to npm.ps1
# and open the wrapper script through the Windows file association.
$process = Start-Process -FilePath $npmPath `
    -ArgumentList $arguments `
    -WorkingDirectory $pcFrontendDir `
    -WindowStyle Hidden `
    -PassThru

Write-Host "[pc-frontend-dev] started PID=$($process.Id)" -ForegroundColor Green

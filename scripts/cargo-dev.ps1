<#
.SYNOPSIS
    Run Cargo validation with a shared development target directory and a writer lock.

.DESCRIPTION
    Daily AI validation should reuse a persistent dev target for speed, but Cargo
    commands must not write the same target directory concurrently. This wrapper
    resolves a machine-local target directory, sets CARGO_TARGET_DIR for the child
    cargo process, and serializes writes with a lock directory.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml pc_lightweight
#>
param(
    [string]$TargetDir,
    [switch]$NoLock,
    [int]$LockTimeoutSeconds = 3600,
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs = @()
)

$ErrorActionPreference = "Stop"

if ($CargoArgs.Count -eq 0) {
    Write-Error "Usage: powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 <cargo-args...>"
}

$gitRoot = git -C $PSScriptRoot rev-parse --show-toplevel 2>$null
if (-not $gitRoot) {
    $gitRoot = git rev-parse --show-toplevel 2>$null
}
if (-not $gitRoot) {
    Write-Error "Current directory is not inside a Git repository."
}
$RepoRoot = $gitRoot.Trim()

function Import-LocalEnvFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) { return }

    foreach ($line in Get-Content -LiteralPath $Path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) {
            continue
        }
        if ($trimmed -notmatch '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$') {
            continue
        }

        $name = $Matches[1]
        $value = $Matches[2].Trim()
        if ($value.Length -ge 2) {
            $first = $value.Substring(0, 1)
            $last = $value.Substring($value.Length - 1, 1)
            if (($first -eq '"' -and $last -eq '"') -or ($first -eq "'" -and $last -eq "'")) {
                $value = $value.Substring(1, $value.Length - 2)
            }
        }

        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

function Resolve-DevCargoTargetDir {
    param(
        [string]$RepoRoot,
        [string]$ExplicitTargetDir
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitTargetDir)) {
        $target = $ExplicitTargetDir.Trim()
        $source = "-TargetDir"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:ELON_DEV_CARGO_TARGET_DIR)) {
        $target = $env:ELON_DEV_CARGO_TARGET_DIR.Trim()
        $source = "ELON_DEV_CARGO_TARGET_DIR"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
        $target = $env:CARGO_TARGET_DIR.Trim()
        $source = "CARGO_TARGET_DIR"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $target = Join-Path $env:LOCALAPPDATA "Elon\build-target\elon-dev-cargo"
        $source = "default LOCALAPPDATA"
    } else {
        $target = Join-Path (Split-Path $RepoRoot -Parent) ".elon-dev-cargo-target"
        $source = "default repo parent"
    }

    if (-not [System.IO.Path]::IsPathRooted($target)) {
        Write-Error "$source must be an absolute path, current value: $target"
    }

    $fullPath = [System.IO.Path]::GetFullPath($target)
    $pathRoot = [System.IO.Path]::GetPathRoot($fullPath)
    if ($pathRoot -and -not (Test-Path -LiteralPath $pathRoot)) {
        Write-Error "Dev Cargo target drive/root does not exist: $fullPath"
    }

    New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
    return $fullPath
}

function Lock-Directory {
    param(
        [string]$Path,
        [int]$TimeoutSeconds
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ($true) {
        try {
            New-Item -ItemType Directory -Path $Path -ErrorAction Stop | Out-Null
            $content = "pid=$PID`nstarted_utc=$([DateTime]::UtcNow.ToString("o"))`n"
            Set-Content -LiteralPath (Join-Path $Path "owner") -Value $content -NoNewline -Encoding UTF8
            return $Path
        } catch {
            if ([DateTime]::UtcNow -ge $deadline) {
                $owner = Join-Path $Path "owner"
                if (Test-Path -LiteralPath $owner) {
                    Write-Host (Get-Content -LiteralPath $owner -Raw)
                }
                throw "Timed out waiting for Cargo dev target lock: $Path"
            }
            Start-Sleep -Seconds 2
        }
    }
}

Import-LocalEnvFile -Path (Join-Path $RepoRoot ".env.local")

$ResolvedTargetDir = Resolve-DevCargoTargetDir -RepoRoot $RepoRoot -ExplicitTargetDir $TargetDir
$LockPath = Join-Path $ResolvedTargetDir ".cargo-dev.lockdir"
$lockDir = $null
$oldCargoTargetDir = $env:CARGO_TARGET_DIR
$hadCargoTargetDir = Test-Path Env:CARGO_TARGET_DIR

try {
    if (-not $NoLock) {
        Write-Host "Waiting for Cargo dev target lock: $LockPath"
        $lockDir = Lock-Directory -Path $LockPath -TimeoutSeconds $LockTimeoutSeconds
    }

    $env:CARGO_TARGET_DIR = $ResolvedTargetDir
    Write-Host "CARGO_TARGET_DIR=$ResolvedTargetDir"
    Write-Host "cargo $($CargoArgs -join ' ')"
    & cargo @CargoArgs
    exit $LASTEXITCODE
} finally {
    if ($lockDir) {
        $owner = Join-Path $lockDir "owner"
        $ownerText = if (Test-Path -LiteralPath $owner) { Get-Content -LiteralPath $owner -Raw } else { "" }
        if ($ownerText -match "pid=$PID") {
            Remove-Item -LiteralPath $lockDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    if ($hadCargoTargetDir) {
        $env:CARGO_TARGET_DIR = $oldCargoTargetDir
    } else {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    }
}

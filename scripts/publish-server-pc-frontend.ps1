function Invoke-LoggedCmd {
    param([string]$Command)

    $previousErrorAction = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & cmd /c $Command 2>&1
        $exitCode = $LASTEXITCODE
        foreach ($line in $output) {
            Write-Host $line
        }
        return $exitCode
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
}

function Invoke-PcFrontendLocalBuild {
    param([string]$FrontendDir)
    $tscCmd = Join-Path $FrontendDir "node_modules\.bin\tsc.cmd"
    $viteCmd = Join-Path $FrontendDir "node_modules\.bin\vite.cmd"
    if (-not (Test-Path $tscCmd) -or -not (Test-Path $viteCmd)) {
        throw "local node_modules is missing tsc/vite"
    }
    Write-Host "   npm build failed, trying node_modules/.bin/tsc + vite ..." -ForegroundColor Gray
    $exitCode = Invoke-LoggedCmd -Command "`"$tscCmd`" --noEmit && `"$viteCmd`" build"
    if ($exitCode -ne 0) {
        throw "node_modules/.bin build failed, exit=$exitCode"
    }
}

function Reset-PcFrontendBuildArtifacts {
    param([string]$FrontendDir)

    foreach ($relative in @("dist", "node_modules\.vite")) {
        $target = Join-Path $FrontendDir $relative
        if (-not (Test-Path $target)) { continue }

        $root = [System.IO.Path]::GetFullPath($FrontendDir)
        $resolved = [System.IO.Path]::GetFullPath($target)
        if (-not $resolved.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to clean unexpected PC frontend path: $resolved"
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}

function Resolve-PnpmCommand {
    if (-not [string]::IsNullOrWhiteSpace($env:PNPM_CMD) -and (Test-Path $env:PNPM_CMD -PathType Leaf)) {
        $configuredExtension = [System.IO.Path]::GetExtension($env:PNPM_CMD)
        if ($configuredExtension -ieq ".ps1") {
            $configuredShim = [System.IO.Path]::ChangeExtension($env:PNPM_CMD, ".cmd")
            if (Test-Path $configuredShim -PathType Leaf) {
                return $configuredShim
            }
            throw "PNPM_CMD points to a PowerShell script that cmd.exe cannot execute: $env:PNPM_CMD"
        }
        return $env:PNPM_CMD
    }

    # Invoke-LoggedCmd runs through cmd.exe. Resolve the Windows command shim
    # explicitly so PowerShell does not hand pnpm.ps1 to the Windows file
    # association (which opens it in Notepad instead of executing pnpm).
    $pnpmCmd = Get-Command "pnpm.cmd" -ErrorAction SilentlyContinue
    if ($pnpmCmd -and $pnpmCmd.Source -and (Test-Path $pnpmCmd.Source -PathType Leaf)) {
        return $pnpmCmd.Source
    }

    $pnpm = Get-Command "pnpm" -ErrorAction SilentlyContinue
    if ($pnpm -and $pnpm.Source -and [System.IO.Path]::GetExtension($pnpm.Source) -ine ".ps1") {
        return $pnpm.Source
    }

    $codexPnpm = Join-Path $HOME ".cache\codex-runtimes\codex-primary-runtime\dependencies\bin\pnpm.cmd"
    if (Test-Path $codexPnpm) {
        return $codexPnpm
    }

    return $null
}

function Invoke-PcFrontendPnpmBuild {
    param([string]$FrontendDir)

    $pnpmCmd = Resolve-PnpmCommand
    if (-not $pnpmCmd) {
        throw "pnpm is unavailable"
    }

    Write-Host "   npm is unavailable, building PC frontend with pnpm ..." -ForegroundColor Gray
    Push-Location $FrontendDir
    try {
        $installExit = Invoke-LoggedCmd -Command "`"$pnpmCmd`" install --no-frozen-lockfile --config.dangerously-allow-all-builds=true"
        if ($installExit -ne 0) { throw "pnpm install failed, exit=$installExit" }

        Reset-PcFrontendBuildArtifacts -FrontendDir $FrontendDir
        $buildExit = Invoke-LoggedCmd -Command "`"$pnpmCmd`" run build"
        if ($buildExit -ne 0) { throw "pnpm run build failed, exit=$buildExit" }
    } finally {
        Pop-Location
    }
}

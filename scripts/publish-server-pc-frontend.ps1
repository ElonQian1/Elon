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
        throw "本地 node_modules 缺少 tsc/vite"
    }
    Write-Host "   🔁 npm 构建失败，尝试直接使用 node_modules/.bin/tsc + vite ..." -ForegroundColor Gray
    $exitCode = Invoke-LoggedCmd -Command "`"$tscCmd`" --noEmit && `"$viteCmd`" build"
    if ($exitCode -ne 0) {
        throw "node_modules/.bin 构建失败，exit=$exitCode"
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
    if (-not [string]::IsNullOrWhiteSpace($env:PNPM_CMD) -and (Test-Path $env:PNPM_CMD)) {
        return $env:PNPM_CMD
    }

    $pnpm = Get-Command "pnpm" -ErrorAction SilentlyContinue
    if ($pnpm) {
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
        throw "pnpm 不可用"
    }

    Write-Host "   🔁 npm 不可用，改用 pnpm 构建 PC 前端 ..." -ForegroundColor Gray
    Push-Location $FrontendDir
    try {
        $installExit = Invoke-LoggedCmd -Command "`"$pnpmCmd`" install --no-frozen-lockfile --config.dangerously-allow-all-builds=true"
        if ($installExit -ne 0) { throw "pnpm install 失败，exit=$installExit" }

        Reset-PcFrontendBuildArtifacts -FrontendDir $FrontendDir
        $buildExit = Invoke-LoggedCmd -Command "`"$pnpmCmd`" run build"
        if ($buildExit -ne 0) { throw "pnpm run build 失败，exit=$buildExit" }
    } finally {
        Pop-Location
    }
}

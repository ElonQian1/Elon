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

function Publish-StaticDist {
    param(
        [string]$LocalDir,
        [string]$RemoteDir,
        [string]$Label,
        [switch]$Required
    )
    if (-not $LocalDir -or -not (Test-Path (Join-Path $LocalDir "index.html"))) {
        Write-Host "3.5⃣  ⚠️  $Label 不存在，跳过上传" -ForegroundColor Yellow
        if ($Required) { throw "$Label 不存在，发布批次失败关闭" }
        return $false
    }

    Write-Host "3.5⃣  上传 $Label 到服务器 $RemoteDir ..." -ForegroundColor Yellow
    $stagingDist = "$RemoteDir-staging-$Sha"
    ssh @SshOpts $Server "mkdir -p '$stagingDist'" 2>&1 | Out-Null
    scp @SshOpts -r "$LocalDir/." "${Server}:${stagingDist}"
    if ($LASTEXITCODE -ne 0) {
        ssh @SshOpts $Server "rm -rf '$stagingDist'" 2>&1 | Out-Null
        Write-Host "   ⚠️  $Label 上传失败（不中止后端部署）" -ForegroundColor Yellow
        if ($Required) { throw "$Label 上传失败，发布批次失败关闭" }
        return $false
    }

    $swapScript = "rm -rf '$RemoteDir' && mv '$stagingDist' '$RemoteDir'"
    ssh @SshOpts $Server $swapScript 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ✅ $Label 上传并替换完成 → $RemoteDir" -ForegroundColor Green
        return $true
    }

    ssh @SshOpts $Server "rm -rf '$stagingDist'" 2>&1 | Out-Null
    Write-Host "   ⚠️  $Label 目录替换失败（staging 已清理）" -ForegroundColor Yellow
    if ($Required) { throw "$Label 目录替换失败，发布批次失败关闭" }
    return $false
}

function Write-GitTextFile {
    param(
        [string]$Commit,
        [string]$GitPath,
        [string]$Destination
    )
    $savedEnc = [Console]::OutputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $content = & git -C $RepoRoot show "${Commit}:$GitPath"
    [Console]::OutputEncoding = $savedEnc
    if ($LASTEXITCODE -ne 0) { throw "无法从 $Commit 导出 $GitPath" }
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllLines($Destination, [string[]]$content, $utf8NoBom)
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

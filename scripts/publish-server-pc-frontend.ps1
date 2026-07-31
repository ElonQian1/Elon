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
    $networkOptions = @('-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1')
    $prepare = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label "$Label prepare" `
        -ArgumentList (@($SshOpts) + @('-n') + $networkOptions + @($Server, "mkdir -p '$stagingDist'"))
    Assert-ElonNativeCommand -Result $prepare -FailureMessage "$Label staging directory creation failed."

    $upload = Invoke-ElonNativeCommand -FilePath 'scp.exe' -TimeoutSeconds 300 -Label "$Label upload" `
        -ArgumentList (@($SshOpts) + $networkOptions + @('-r', "$LocalDir/.", "${Server}:${stagingDist}"))
    if ($upload.ExitCode -ne 0) {
        Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label "$Label cleanup" `
            -ArgumentList (@($SshOpts) + @('-n') + $networkOptions + @($Server, "rm -rf '$stagingDist'")) | Out-Null
        Write-Host "   ⚠️  $Label 上传失败（不中止后端部署）" -ForegroundColor Yellow
        if ($Required) { throw "$Label 上传失败，发布批次失败关闭" }
        return $false
    }

    # Publish hashed assets additively before atomically replacing index.html.
    # Old HTML can therefore keep loading its old hashes throughout the deploy.
    # A marker defers the first GC for a full grace window; later runs remove
    # only assets that have been untouched for at least 14 days.
    $swapScript = "set -eu; mkdir -p '$RemoteDir/assets'; " +
        "if [ -f '$RemoteDir/index.html' ]; then { grep -oE 'assets/[A-Za-z0-9._/-]+' '$RemoteDir/index.html' || true; } | " +
        "sed 's#^assets/##' | while IFS= read -r asset; do [ ! -f '$RemoteDir/assets/'`"`$asset`" ] || touch '$RemoteDir/assets/'`"`$asset`"; done; fi; " +
        "if [ -d '$stagingDist/assets' ]; then cp -a '$stagingDist/assets/.' '$RemoteDir/assets/'; fi; " +
        "for item in '$stagingDist'/*; do [ -e `"`$item`" ] || continue; base=`$(basename `"`$item`"); " +
        "[ `"`$base`" = assets ] && continue; [ `"`$base`" = index.html ] && continue; " +
        "if [ -f `"`$item`" ]; then cp `"`$item`" '$RemoteDir/.publish-new-'`"`$base`"; mv -f '$RemoteDir/.publish-new-'`"`$base`" '$RemoteDir/'`"`$base`"; fi; done; " +
        "cp '$stagingDist/index.html' '$RemoteDir/.publish-new-index-$Sha'; " +
        "mv -f '$RemoteDir/.publish-new-index-$Sha' '$RemoteDir/index.html'; " +
        "if [ ! -f '$RemoteDir/.atomic-static-retention' ]; then touch '$RemoteDir/.atomic-static-retention'; " +
        "elif find '$RemoteDir/.atomic-static-retention' -mtime +14 -print -quit | grep -q .; then " +
        "find '$RemoteDir/assets' -type f -mtime +14 -delete; touch '$RemoteDir/.atomic-static-retention'; fi; " +
        "rm -rf '$stagingDist'"
    $swap = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 90 -Label "$Label atomic swap" `
        -ArgumentList (@($SshOpts) + @('-n') + $networkOptions + @($Server, $swapScript))
    if ($swap.ExitCode -eq 0) {
        Write-Host "   ✅ $Label 原子入口发布完成（旧 hash 保留宽限期）→ $RemoteDir" -ForegroundColor Green
        return $true
    }

    Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label "$Label cleanup" `
        -ArgumentList (@($SshOpts) + @('-n') + $networkOptions + @($Server, "rm -rf '$stagingDist'")) | Out-Null
    Write-Host "   ⚠️  $Label 目录替换失败（staging 已清理）" -ForegroundColor Yellow
    if ($Required) { throw "$Label 目录替换失败，发布批次失败关闭" }
    return $false
}

function Export-PcLegacyDist {
    param(
        [string]$Commit,
        [string]$OutDir
    )
    if (Test-Path $OutDir) {
        Remove-Item -LiteralPath $OutDir -Recurse -Force
    }
    $assetsDir = Join-Path $OutDir "assets"
    New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null

    $assetPaths = & git -C $RepoRoot ls-tree -r --name-only $Commit -- "server/src/assets"
    if ($LASTEXITCODE -ne 0) { throw "无法读取旧版 PC 资源列表: $Commit" }
    foreach ($assetPath in $assetPaths) {
        $name = Split-Path $assetPath -Leaf
        if ($name -eq "pc_app.html") { continue }
        if (($name -like "pc_*") -or ($name -eq "voice_tts_sdk.js")) {
            Write-GitTextFile -Commit $Commit -GitPath $assetPath -Destination (Join-Path $assetsDir $name)
        }
    }

    $savedEnc = [Console]::OutputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $htmlLines = & git -C $RepoRoot show "${Commit}:server/src/assets/pc_app.html"
    [Console]::OutputEncoding = $savedEnc
    if ($LASTEXITCODE -ne 0) { throw "无法导出旧版 PC HTML: $Commit" }
    $html = [string]::Join("`n", [string[]]$htmlLines)
    $brandPath = Join-Path $RepoRoot "server/src/assets/ic_app_brand.b64"
    $brandB64 = if (Test-Path $brandPath) { (Get-Content -LiteralPath $brandPath -Raw).Trim() } else { "" }
    $html = $html.Replace("__BRAND_PNG_B64__", $brandB64)
    $html = $html.Replace('"/assets/', '"/pc-legacy/assets/')
    $html = $html.Replace("'/assets/", "'/pc-legacy/assets/")
    $openNewLink = '<a class="text-button pc-legacy-new-link" id="pcLegacyOpenNewBtn" href="/pc" title="打开新版 PC 工作台" aria-label="打开新版 PC 工作台" style="display:inline-flex;align-items:center;">打开新版</a>'
    if ($html -match 'id="openLegacyWebBtn"') {
        $html = $html -replace '(id="openLegacyWebBtn"[^>]*>[^<]*</button>)', "`$1`n          $openNewLink"
    } elseif ($html -match '<div class="topbar-actions">') {
        $html = $html -replace '(<div class="topbar-actions">)', "`$1`n          $openNewLink"
    } else {
        Write-Host "   ℹ️  旧版 PC HTML 未找到 openLegacyWebBtn，跳过注入打开新版入口"
    }
    $legacySwitchScriptTag = '    <script src="/pc-legacy/assets/pc_legacy_switch.js"></script>'
    if (-not $html.Contains("</body>")) {
        throw "旧版 PC HTML 中未找到 </body>，无法注入新版切换脚本"
    }
    $html = $html.Replace("</body>", "$legacySwitchScriptTag`n  </body>")

    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    $legacySwitchJs = @'
(function () {
  function legacyToken() {
    return localStorage.getItem('lodex_token') || localStorage.getItem('elon_token') || '';
  }

  function bridgeToken() {
    var token = legacyToken();
    if (!token) return;
    try {
      localStorage.setItem('elon_auth', JSON.stringify({
        state: { token: token, user: null },
        version: 0
      }));
    } catch (_) {
      // Keep normal navigation even when localStorage is unavailable.
    }
  }

  var btn = document.getElementById('pcLegacyOpenNewBtn');
  if (btn) btn.addEventListener('click', bridgeToken);
})();
'@
    [System.IO.File]::WriteAllText((Join-Path $OutDir "index.html"), $html, $utf8NoBom)
    [System.IO.File]::WriteAllText((Join-Path $assetsDir "pc_legacy_switch.js"), $legacySwitchJs, $utf8NoBom)
    return $OutDir
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

function Invoke-PcFrontendBundleBudget {
    param([string]$FrontendDir)

    $repoRoot = [System.IO.Path]::GetFullPath((Join-Path $FrontendDir ".."))
    $distDir = Join-Path $FrontendDir "dist"
    $budgetScript = Join-Path $repoRoot "scripts\check-pc-frontend-bundle-budget.js"
    if (-not (Test-Path (Join-Path $distDir "index.html"))) {
        throw "PC frontend dist is missing index.html: $distDir"
    }
    if (-not (Test-Path $budgetScript)) {
        throw "PC frontend bundle budget script is missing: $budgetScript"
    }
    if (-not (Get-Command "node" -ErrorAction SilentlyContinue)) {
        throw "node is unavailable for the PC frontend bundle budget check"
    }

    Write-Host "   Checking PC frontend bundle budget..." -ForegroundColor Gray
    $budgetExit = Invoke-LoggedCmd -Command "node `"$budgetScript`" --dist `"$distDir`""
    if ($budgetExit -ne 0) {
        throw "PC frontend bundle budget check failed, exit=$budgetExit"
    }
    Write-Host "   PC frontend bundle budget passed" -ForegroundColor Green
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

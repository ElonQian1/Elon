Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'apk-adb-discovery.ps1')

function ConvertTo-ElonProcessArgument {
    param([AllowEmptyString()][string]$Value)

    if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') { return $Value }
    $builder = [System.Text.StringBuilder]::new()
    [void]$builder.Append('"')
    $slashes = 0
    foreach ($character in $Value.ToCharArray()) {
        if ($character -eq '\') {
            $slashes++
            continue
        }
        if ($character -eq '"') {
            [void]$builder.Append(('\' * (($slashes * 2) + 1)))
            [void]$builder.Append('"')
        } else {
            if ($slashes -gt 0) { [void]$builder.Append(('\' * $slashes)) }
            [void]$builder.Append($character)
        }
        $slashes = 0
    }
    if ($slashes -gt 0) { [void]$builder.Append(('\' * ($slashes * 2))) }
    [void]$builder.Append('"')
    $builder.ToString()
}

function Invoke-ElonAdbCommand {
    param(
        [Parameter(Mandatory)][string]$AdbPath,
        [Parameter(Mandatory)][string[]]$Arguments,
        [int]$TimeoutSeconds = 30
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $AdbPath
    $startInfo.Arguments = (($Arguments | ForEach-Object { ConvertTo-ElonProcessArgument $_ }) -join ' ')
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        if (-not $process.Start()) { throw "Failed to start ADB: $AdbPath" }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            try { $process.Kill() } catch {}
            throw "ADB command timed out after ${TimeoutSeconds}s: adb $($Arguments -join ' ')"
        }
        $process.WaitForExit()
        $stdout = $stdoutTask.Result.Trim()
        $stderr = $stderrTask.Result.Trim()
        [pscustomobject]@{
            ExitCode = $process.ExitCode
            Stdout = $stdout
            Stderr = $stderr
            Text = (($stdout, $stderr | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join "`n")
        }
    } finally {
        $process.Dispose()
    }
}

function Resolve-ElonApkAdbPath {
    param(
        [string]$ConfiguredPath,
        [string]$RequestedPath
    )

    $candidates = @(
        $RequestedPath,
        $env:ELON_ADB_PATH,
        $ConfiguredPath,
        $(if ($env:ANDROID_HOME) { Join-Path $env:ANDROID_HOME 'platform-tools\adb.exe' }),
        $(if ($env:ANDROID_SDK_ROOT) { Join-Path $env:ANDROID_SDK_ROOT 'platform-tools\adb.exe' }),
        $(if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'Android\Sdk\platform-tools\adb.exe' }),
        'D:\Android\sdk\platform-tools\adb.exe'
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    $command = Get-Command adb -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw 'ADB was not found in PATH or a known Android SDK location.'
}

function Get-ElonApkAdbConfigPath {
    param([string]$ConfigPath)

    if (-not [string]::IsNullOrWhiteSpace($ConfigPath)) { return $ConfigPath }
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_APK_ADB_TARGETS_FILE)) {
        return $env:ELON_APK_ADB_TARGETS_FILE
    }
    Join-Path $env:USERPROFILE '.elon\apk-adb-targets.json'
}

function Test-ElonJsonProperty {
    param([object]$Object, [string]$Name)
    $null -ne $Object.PSObject.Properties[$Name]
}

function Invoke-ElonTargetAdbDeployment {
    param(
        [Parameter(Mandatory)][string]$AdbPath,
        [Parameter(Mandatory)][object]$Target,
        [Parameter(Mandatory)][string]$ApkPath,
        [Parameter(Mandatory)][string]$PackageName,
        [Parameter(Mandatory)][int]$ExpectedVersionCode,
        [Parameter(Mandatory)][int]$MaxAttempts,
        [Parameter(Mandatory)][int]$RetryDelaySeconds,
        [Parameter(Mandatory)][bool]$LaunchAfterInstall
    )

    $serial = "$($Target.serial)".Trim()
    $hardwareSerial = "$($Target.hardwareSerial)".Trim()
    $label = if (Test-ElonJsonProperty $Target 'label') { "$($Target.label)".Trim() } else { $serial }
    if ([string]::IsNullOrWhiteSpace($serial) -or [string]::IsNullOrWhiteSpace($hardwareSerial)) {
        throw 'Every ADB target requires serial and hardwareSerial to prevent deployment to a reused endpoint.'
    }

    $lastError = $null
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            if ($serial -match '^[^:]+:\d+$') {
                $connect = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('connect', $serial) -TimeoutSeconds 15
                if ($connect.ExitCode -ne 0) { throw "adb connect failed: $($connect.Text)" }
            }

            $state = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('-s', $serial, 'get-state') -TimeoutSeconds 10
            if ($state.ExitCode -ne 0 -or $state.Stdout.Trim() -ne 'device') {
                throw "Device is not in the device state: $($state.Text)"
            }

            $identity = Invoke-ElonAdbCommand -AdbPath $AdbPath `
                -Arguments @('-s', $serial, 'shell', 'getprop', 'ro.serialno') -TimeoutSeconds 10
            if ($identity.ExitCode -ne 0 -or $identity.Stdout.Trim() -ine $hardwareSerial) {
                throw "Hardware serial mismatch: expected $hardwareSerial, got $($identity.Stdout.Trim())"
            }

            Write-Host "   [$label] Installing release APK (attempt $attempt/$MaxAttempts)..." -ForegroundColor Cyan
            $before = Invoke-ElonAdbCommand -AdbPath $AdbPath `
                -Arguments @('-s', $serial, 'shell', 'dumpsys', 'package', $PackageName) -TimeoutSeconds 20
            $prior = [regex]::Match($before.Stdout, '\bversionCode=(\d+)')
            if ($before.ExitCode -ne 0) { throw 'Could not check installed version before update.' }
            if ($prior.Success -and [long]$prior.Groups[1].Value -gt $ExpectedVersionCode) {
                return [pscustomobject]@{ Label=$label; Serial=$serial; Status='newer_installed' }
            }
            $install = Invoke-ElonAdbCommand -AdbPath $AdbPath `
                -Arguments @('-s', $serial, 'install', '-r', $ApkPath) -TimeoutSeconds 360
            if ($install.ExitCode -ne 0 -or $install.Text -notmatch '(?im)^Success\s*$') {
                throw "adb install -r did not return Success: $($install.Text)"
            }

            $package = Invoke-ElonAdbCommand -AdbPath $AdbPath `
                -Arguments @('-s', $serial, 'shell', 'dumpsys', 'package', $PackageName) -TimeoutSeconds 20
            $versionMatch = [regex]::Match($package.Text, '(?m)\bversionCode=(\d+)')
            if ($package.ExitCode -ne 0 -or -not $versionMatch.Success) {
                throw "Could not read the installed versionCode for ${PackageName}: $($package.Text)"
            }
            $installedVersionCode = [int]$versionMatch.Groups[1].Value
            if ($installedVersionCode -ne $ExpectedVersionCode) {
                throw "Installed version mismatch: expected build $ExpectedVersionCode, got build $installedVersionCode"
            }

            if ($LaunchAfterInstall) {
                Invoke-ElonAdbCommand -AdbPath $AdbPath `
                    -Arguments @('-s', $serial, 'shell', 'am', 'force-stop', $PackageName) -TimeoutSeconds 15 | Out-Null
                $launch = Invoke-ElonAdbCommand -AdbPath $AdbPath `
                    -Arguments @('-s', $serial, 'shell', 'monkey', '-p', $PackageName, '-c', 'android.intent.category.LAUNCHER', '1') `
                    -TimeoutSeconds 30
                if ($launch.ExitCode -ne 0) { throw "Install succeeded but automatic launch failed: $($launch.Text)" }
            }

            Write-Host "   [$label] Unattended update verified at build $ExpectedVersionCode" -ForegroundColor Green
            return [pscustomobject]@{ Label = $label; Serial = $serial; Status = 'updated' }
        } catch {
            $lastError = $_.Exception.Message
            if ($attempt -lt $MaxAttempts) {
                Write-Warning "   [$label] ADB update failed; retrying in ${RetryDelaySeconds}s: $lastError"
                Start-Sleep -Seconds $RetryDelaySeconds
            }
        }
    }
    throw "[$label] ADB update failed after $MaxAttempts attempts: $lastError"
}

function Invoke-ElonApkAdbAutodeploy {
    param(
        [Parameter(Mandatory)][string]$ApkPath,
        [Parameter(Mandatory)][int]$ExpectedVersionCode,
        [string]$PackageName = 'com.elon.app',
        [string]$ConfigPath,
        [string]$AdbPath,
        [string]$ReceiptPath
    )

    $resolvedConfigPath = Get-ElonApkAdbConfigPath -ConfigPath $ConfigPath
    if (-not (Test-Path -LiteralPath $ApkPath -PathType Leaf)) { throw "APK does not exist: $ApkPath" }
    if ($ConfigPath -or $env:ELON_APK_ADB_TARGETS_FILE) {
        if (-not (Test-Path -LiteralPath $resolvedConfigPath -PathType Leaf)) {
            throw 'EXPLICIT_ADB_CONFIG_MISSING: requested device configuration does not exist.'
        }
        $config = Get-Content -LiteralPath $resolvedConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
    } else {
        $config = Get-ElonProjectApkAdbConfig
    }
    if ((Test-ElonJsonProperty $config 'enabled') -and -not [bool]$config.enabled) {
        throw 'ADB_CHECK_DISABLED: release phone checks are mandatory; enable the configured device checks.'
    }
    if ((Test-ElonJsonProperty $config 'schemaVersion') -and [int]$config.schemaVersion -ne 1) {
        throw "Unsupported ADB autodeploy schemaVersion: $($config.schemaVersion)"
    }
    $targets = @($config.targets | Where-Object {
        -not (Test-ElonJsonProperty $_ 'enabled') -or [bool]$_.enabled
    })
    foreach ($target in $targets) {
        if (-not (Test-ElonJsonProperty $target 'label')) { $target | Add-Member NoteProperty label ([string]$target.hardwareSerial) }
    }
    if ($targets.Count -eq 0) { throw "ADB autodeploy is enabled but targets is empty: $resolvedConfigPath" }

    $configuredAdbPath = if (Test-ElonJsonProperty $config 'adbPath') { [string]$config.adbPath } else { '' }
    $resolvedAdbPath = Resolve-ElonApkAdbPath -ConfiguredPath $configuredAdbPath -RequestedPath $AdbPath
    $effectivePackageName = if (Test-ElonJsonProperty $config 'packageName') {
        [string]$config.packageName
    } else { $PackageName }
    $maxAttempts = if (Test-ElonJsonProperty $config 'maxAttempts') { [int]$config.maxAttempts } else { 3 }
    $retryDelaySeconds = if (Test-ElonJsonProperty $config 'retryDelaySeconds') { [int]$config.retryDelaySeconds } else { 5 }
    $launchAfterInstall = if (Test-ElonJsonProperty $config 'launchAfterInstall') { [bool]$config.launchAfterInstall } else { $true }
    if ($maxAttempts -lt 1 -or $maxAttempts -gt 5) { throw 'maxAttempts must be between 1 and 5.' }
    if ($retryDelaySeconds -lt 0 -or $retryDelaySeconds -gt 60) { throw 'retryDelaySeconds must be between 0 and 60.' }

    Write-Host "Deploying the release APK to $($targets.Count) whitelisted ADB target(s)..." -ForegroundColor Cyan
    $results = @()
    $failures = @()
    foreach ($target in @($targets | Sort-Object hardwareSerial -Unique)) {
        try {
            $transport = Find-ElonApkAdbTransport -AdbPath $resolvedAdbPath -Target $target
            if ($transport.Status -ne 'online') {
                $results += [pscustomobject]@{ Label=$target.label; HardwareSerial=$target.hardwareSerial; Serial=''; Status=$transport.Status }
                Write-Host "APK_ADB_TARGET=$($target.label) STATUS=$($transport.Status)"
                if ($transport.Status -in @('identity_mismatch','probe_failed','unauthorized')) { $failures += "$($target.label): $($transport.Status)" }
                continue
            }
            $target = [pscustomobject]@{ label=$target.label; hardwareSerial=$target.hardwareSerial; serial=$transport.Serial }
            $installed = Invoke-ElonTargetAdbDeployment -AdbPath $resolvedAdbPath -Target $target `
                -ApkPath $ApkPath -PackageName $effectivePackageName -ExpectedVersionCode $ExpectedVersionCode `
                -MaxAttempts $maxAttempts -RetryDelaySeconds $retryDelaySeconds -LaunchAfterInstall $launchAfterInstall
            $installed | Add-Member NoteProperty HardwareSerial $target.hardwareSerial
            $results += $installed
        } catch {
            $failures += $_.Exception.Message
            $results += [pscustomobject]@{ Label=$target.label; HardwareSerial=$target.hardwareSerial; Serial=''; Status='failed' }
        }
    }
    if (-not $ReceiptPath) {
        $ReceiptPath = Join-Path $env:USERPROFILE ('.elon/apk-adb-receipts/' + $PackageName + '-' + $ExpectedVersionCode + '-' + [guid]::NewGuid().ToString('N') + '.json')
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $ReceiptPath) -Force | Out-Null
    @{ schema='elon.apk_adb_receipt.v1'; checkedAt=[DateTime]::UtcNow.ToString('o');
        packageName=$effectivePackageName; expectedVersionCode=$ExpectedVersionCode;
        apkSha256=(Get-FileHash -LiteralPath $ApkPath -Algorithm SHA256).Hash.ToLowerInvariant();
        targets=@($results); failures=@($failures) } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $ReceiptPath -Encoding UTF8
    Write-Host "APK_ADB_RECEIPT=$ReceiptPath"
    if ($failures.Count -gt 0) {
        Write-Host 'APK_ADB_DEPLOY_STATUS=failed' -ForegroundColor Red
        throw ("The server APK was published, but one or more whitelisted ADB deployments failed: " + ($failures -join ' | '))
    }
    $offline = @($results | Where-Object { $_.Status -eq 'offline' }).Count
    $overall = if ($offline -eq $results.Count) { 'offline' } elseif ($offline -gt 0) { 'partial_offline' } else { 'updated' }
    Write-Host "APK_ADB_DEPLOY_STATUS=$overall"
    foreach ($result in $results) { Write-Host "APK_ADB_TARGET=$($result.Label) STATUS=$($result.Status)" }
    $results
}

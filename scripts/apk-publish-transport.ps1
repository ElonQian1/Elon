function Assert-RemoteApkArtifact {
    param(
        [Parameter(Mandatory)] [string]$ExpectedSha256,
        [Parameter(Mandatory)] [int64]$ExpectedSize
    )

    $options = Get-ElonApkSshOptions
    $artifactPath = "$ServerDir/ElonSpeed-latest.apk"
    $command = 'printf ''%s %s'' "$(sha256sum ''__APK_PATH__'' | awk ''{print $1}'')" "$(stat -c %s ''__APK_PATH__'')"'
    $command = $command.Replace('__APK_PATH__', $artifactPath)
    $result = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'verify remote APK' `
        -ArgumentList ($options + @($ServerHost, $command))
    Assert-ElonNativeCommand -Result $result -FailureMessage 'Remote APK hash verification failed.'
    $parts = $result.Stdout.Trim() -split '\s+'
    if ($parts.Count -lt 2 -or $parts[0] -ne $ExpectedSha256 -or [int64]$parts[1] -ne $ExpectedSize) {
        throw "Remote APK differs from the local artifact: remote=$($result.Stdout.Trim()) expected=$ExpectedSha256 $ExpectedSize"
    }
    Write-Host '   Remote APK SHA-256 and size verified.' -ForegroundColor Green
}

function Get-ElonApkSshOptions {
    @(
        '-n', '-o', 'ProxyCommand=none', '-o', 'BatchMode=yes',
        '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=5',
        '-o', 'ServerAliveCountMax=1'
    )
}

function Get-ElonApkScpOptions {
    @(
        '-o', 'ProxyCommand=none', '-o', 'BatchMode=yes',
        '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=5',
        '-o', 'ServerAliveCountMax=1'
    )
}

function Remove-ElonApkStaging {
    param([string]$ApkStage, [string]$JsonStage, [string]$Label = 'cleanup APK staging')

    Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label $Label `
        -ArgumentList ((Get-ElonApkSshOptions) + @($ServerHost, "rm -f '$ApkStage' '$JsonStage'")) | Out-Null
}

function Publish-ApkStaged {
    param(
        [string]$ApkPath,
        [string]$JsonPath,
        [string]$ReleaseSha,
        [string]$ExpectedServerSha,
        [string]$ExpectedSha256,
        [int]$Attempt = 1
    )

    $apkStage = "$ServerDir/ElonSpeed-latest.apk.$ReleaseSha.tmp"
    $jsonStage = "$ServerDir/version.json.$ReleaseSha.tmp"
    $sshOptions = Get-ElonApkSshOptions
    $scpOptions = Get-ElonApkScpOptions
    $prepare = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'prepare APK staging' `
        -ArgumentList ($sshOptions + @($ServerHost, "mkdir -p $ServerDir"))
    Assert-ElonNativeCommand -Result $prepare -FailureMessage "Unable to create remote APK directory: $ServerDir"

    $apkUpload = Invoke-ElonNativeCommand -FilePath 'scp.exe' -TimeoutSeconds 300 -Label 'upload APK staging' `
        -ArgumentList ($scpOptions + @($ApkPath, "${ServerHost}:${apkStage}"))
    Assert-ElonNativeCommand -Result $apkUpload -FailureMessage 'APK staging upload failed.'
    Write-Host '   APK staging upload completed.' -ForegroundColor Green

    $jsonUpload = Invoke-ElonNativeCommand -FilePath 'scp.exe' -TimeoutSeconds 60 -Label 'upload APK version metadata' `
        -ArgumentList ($scpOptions + @($JsonPath, "${ServerHost}:${jsonStage}"))
    Assert-ElonNativeCommand -Result $jsonUpload -FailureMessage 'version.json staging upload failed.'
    Write-Host '   version.json staging upload completed.' -ForegroundColor Green

    $remoteScript = New-ElonApkAtomicDeployScript -ApkStage $apkStage -JsonStage $jsonStage `
        -ReleaseSha $ReleaseSha -ExpectedServerSha $ExpectedServerSha -ExpectedSha256 $ExpectedSha256
    $remoteScriptB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($remoteScript))
    $deployCommand = "echo '$remoteScriptB64' | base64 -d | bash"
    $deploy = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 90 -Label 'atomic APK deploy' `
        -ArgumentList ($sshOptions + @($ServerHost, $deployCommand))

    if ($deploy.ExitCode -eq 42) {
        Resolve-ElonApkCasConflict -ApkPath $ApkPath -JsonPath $JsonPath -ReleaseSha $ReleaseSha `
            -ExpectedSha256 $ExpectedSha256 -ApkStage $apkStage -JsonStage $jsonStage -Attempt $Attempt
        return
    }
    if ($deploy.ExitCode -ne 0) {
        Complete-Release -Success:$false -ErrorMessage "apk atomic deploy failed: exit=$($deploy.ExitCode)"
        Assert-ElonNativeCommand -Result $deploy -FailureMessage 'Atomic APK deployment failed.'
    }
}

function New-ElonApkAtomicDeployScript {
    param(
        [string]$ApkStage,
        [string]$JsonStage,
        [string]$ReleaseSha,
        [string]$ExpectedServerSha,
        [string]$ExpectedSha256
    )

    # Windows PowerShell 5 can misparse shell operators inside a here-string in
    # some mixed-line-ending worktrees. An explicit line array keeps this
    # release-critical script portable across powershell.exe and pwsh.
    $template = @(
        'set -eu'
        "APP_DIR='__APP_DIR__'"
        "EXPECTED='__EXPECTED__'"
        "NEW_SHA='__NEW_SHA__'"
        "APK_STAGE='__APK_STAGE__'"
        "JSON_STAGE='__JSON_STAGE__'"
        "EXPECTED_HASH='__EXPECTED_HASH__'"
        'LOCK_FILE="$APP_DIR/.apk-deploy.lock"'
        'SHA_FILE="$APP_DIR/.apk-deployed-sha"'
        '('
        '  flock -x 9'
        '  CURRENT=""'
        '  if [ -f "$SHA_FILE" ]; then CURRENT="$(cat "$SHA_FILE" 2>/dev/null)"; fi'
        '  if [ "$CURRENT" != "$EXPECTED" ]; then'
        '    echo "APK_DEPLOY_CAS_MISMATCH current=$CURRENT expected=$EXPECTED" >&2'
        '    exit 42'
        '  fi'
        '  ACTUAL_HASH="$(sha256sum "$APK_STAGE" | awk ''{print $1}'')"'
        '  if [ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]; then'
        '    echo "APK_STAGE_HASH_MISMATCH actual=$ACTUAL_HASH expected=$EXPECTED_HASH" >&2'
        '    exit 43'
        '  fi'
        '  mv "$APK_STAGE" "$APP_DIR/ElonSpeed-latest.apk"'
        '  mv "$JSON_STAGE" "$APP_DIR/version.json"'
        '  printf ''%s\n'' "$NEW_SHA" > "$SHA_FILE"'
        ') 9>"$LOCK_FILE"'
    ) -join "`n"
    $template.
        Replace('__APP_DIR__', $ServerDir).
        Replace('__EXPECTED__', $ExpectedServerSha).
        Replace('__NEW_SHA__', $ReleaseSha).
        Replace('__APK_STAGE__', $ApkStage).
        Replace('__JSON_STAGE__', $JsonStage).
        Replace('__EXPECTED_HASH__', $ExpectedSha256).
        Replace("`r`n", "`n").
        Replace("`r", "`n")
}

function Resolve-ElonApkCasConflict {
    param(
        [string]$ApkPath,
        [string]$JsonPath,
        [string]$ReleaseSha,
        [string]$ExpectedSha256,
        [string]$ApkStage,
        [string]$JsonStage,
        [int]$Attempt
    )

    $deployedSha = Get-DeployedApkSha
    if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
        Write-Host "A newer APK is already deployed: $((Format-ShortSha $deployedSha)). This staging upload will not overwrite it." -ForegroundColor Cyan
        Remove-ElonApkStaging -ApkStage $ApkStage -JsonStage $JsonStage -Label 'cleanup superseded APK staging'
        Complete-Release -Success:$false -ErrorMessage "superseded by deployed apk $deployedSha"
        Write-ApkPublishStatus -ApkReleaseStatus 'published' -Message 'A newer mainline APK is already deployed; staging did not overwrite it.'
        exit 0
    }
    if ($deployedSha -and (Test-GitAncestor $deployedSha $ReleaseSha) -and $Attempt -lt 3) {
        Write-Host "An older APK $((Format-ShortSha $deployedSha)) was just deployed; retrying the newer release $((Format-ShortSha $ReleaseSha))." -ForegroundColor Cyan
        Publish-ApkStaged -ApkPath $ApkPath -JsonPath $JsonPath -ReleaseSha $ReleaseSha `
            -ExpectedServerSha $deployedSha -ExpectedSha256 $ExpectedSha256 -Attempt ($Attempt + 1)
        return
    }
    Remove-ElonApkStaging -ApkStage $ApkStage -JsonStage $JsonStage
    Complete-Release -Success:$false -ErrorMessage 'cas mismatch in apk deploy'
    Write-Host 'APK deployment CAS failed because remote state changed. Staging did not overwrite it; publication is delegated to the latest mainline.' -ForegroundColor Cyan
    Write-ApkPublishStatus -ApkReleaseStatus 'superseded_by_newer_main' -Message 'Code is merged; publication is delegated to the latest mainline.'
    exit 0
}

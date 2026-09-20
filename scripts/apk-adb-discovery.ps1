Set-StrictMode -Version Latest

function Get-ElonProjectApkAdbConfig {
    # Fixed, read-only query of the main project's registered debug devices. No credentials are returned.
    $source = @'
import sqlite3,json
c=sqlite3.connect('file:/opt/elon/data/elon.db?mode=ro',uri=True)
rows=c.execute('select hardware_serial,display_name,last_endpoint from project_android_devices where project_id=?',('elon-self',)).fetchall()
print(json.dumps({'schemaVersion':1,'enabled':True,'launchAfterInstall':False,'maxAttempts':2,'retryDelaySeconds':2,'targets':[{'hardwareSerial':r[0],'label':r[1],'serial':r[2]} for r in rows]}))
'@
    $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($source))
    $remote = "python3 -c `"exec(__import__('base64').b64decode('$encoded'))`""
    $ssh = (Get-Command ssh -ErrorAction Stop).Source
    $result = Invoke-ElonAdbCommand -AdbPath $ssh -Arguments @('-o','BatchMode=yes','-o','ConnectTimeout=10',
        '-o','ProxyCommand=none','-o','ProxyJump=none','root@43.139.149.158',$remote) -TimeoutSeconds 30
    if ($result.ExitCode -ne 0) { throw 'PROJECT_ADB_REGISTRY_UNAVAILABLE: cannot read main-project device records; device checks are incomplete.' }
    $config = $result.Stdout | ConvertFrom-Json
    if (@($config.targets).Count -eq 0) { throw 'PROJECT_ADB_REGISTRY_EMPTY: main project has no registered phones.' }
    Write-Host "APK_ADB_TARGET_SOURCE=main_project_registry count=$(@($config.targets).Count)"
    return $config
}

function Find-ElonApkAdbTransport {
    param([string]$AdbPath, [object]$Target)
    $hardware = [string]$Target.hardwareSerial
    if ($hardware -notmatch '^[A-Za-z0-9._-]+$') { throw 'Invalid registered hardware serial.' }
    $candidates = @($hardware)
    if (Test-ElonJsonProperty $Target 'serial') { $candidates += [string]$Target.serial }
    if (Test-ElonJsonProperty $Target 'serials') { $candidates += @($Target.serials) }
    $listed = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('devices','-l') -TimeoutSeconds 15
    if ($listed.ExitCode -ne 0) { throw 'ADB_DEVICE_DISCOVERY_FAILED' }
    $online = @()
    foreach ($line in ($listed.Stdout -split '\r?\n')) {
        if ($line -match '^(\S+)\s+(device|offline|unauthorized)\b' -and $Matches[1] -notlike 'emulator-*') {
            $online += $Matches[1]
        }
    }
    # Already connected USB transports are tried before network connections.
    $candidates = @($hardware) + @($online | Where-Object { $_ -notmatch ':' }) + $candidates + $online
    try {
        $mdns = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('mdns','services') -TimeoutSeconds 5
        foreach ($line in ($mdns.Stdout -split '\r?\n')) {
            if ($line -match ('^adb-' + [regex]::Escape($hardware) + '-\S+\s+_adb-tls-connect\._tcp\.?\s+(\S+)$')) {
                $candidates += $Matches[1]
            }
        }
    } catch { } # mDNS is optional; explicit endpoints and USB remain usable.
    $reason = 'offline'
    foreach ($serial in @($candidates | Where-Object { $_ -and $_ -notlike 'emulator-*' } | Select-Object -Unique)) {
        if ($serial -notmatch '^[A-Za-z0-9._:-]+$') { continue }
        try {
            if ($serial -match ':\d+$' -and $serial -notin $online) {
                Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('connect',$serial) -TimeoutSeconds 8 | Out-Null
            }
            $state = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('-s',$serial,'get-state') -TimeoutSeconds 5
            if ($state.ExitCode -ne 0 -or $state.Stdout.Trim() -ne 'device') {
                if ($state.Text -match 'unauthorized') { $reason = 'unauthorized' }
                continue
            }
            $identity = Invoke-ElonAdbCommand -AdbPath $AdbPath -Arguments @('-s',$serial,'shell','getprop','ro.serialno') -TimeoutSeconds 5
            if ($identity.ExitCode -ne 0) { $reason = 'probe_failed'; continue }
            if ($identity.Stdout.Trim() -ine $hardware) {
                if ((Test-ElonJsonProperty $Target 'serial') -and $serial -eq $Target.serial) { $reason = 'identity_mismatch' }
                continue
            }
            return [pscustomobject]@{ Serial=$serial; Status='online' }
        } catch { if ($serial -in $online) { $reason = 'probe_failed' } }
    }
    return [pscustomobject]@{ Serial=''; Status=$reason }
}

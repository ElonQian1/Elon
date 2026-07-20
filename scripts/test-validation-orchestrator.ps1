$ErrorActionPreference = "Stop"
$RepoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$Modules = Join-Path $PSScriptRoot "validation"
Import-Module (Join-Path $Modules "Validation.Fingerprint.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Scheduler.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Evidence.psm1") -Force -DisableNameChecking
$script:Assertions = 0
function Assert-True([bool]$Condition,[string]$Message) { $script:Assertions++; if (-not $Condition) { throw "ASSERT FAILED: $Message" } }
function Assert-Equal($Expected,$Actual,[string]$Message) { $script:Assertions++; if ($Expected -ne $Actual) { throw "ASSERT FAILED: $Message expected=$Expected actual=$Actual" } }
function Stop-TestProcess($Process) { if(-not $Process){return}; try{$Process.Refresh();if(-not $Process.HasExited){& cmd.exe /d /c "taskkill /PID $($Process.Id) /T /F >nul 2>&1" | Out-Null}}catch{}finally{$Process.Dispose()} }
$TempRoot = Join-Path $env:TEMP ("elon-validation-test-" + [Guid]::NewGuid().ToString("N"))
$CacheRoot = Join-Path $TempRoot "cache"; $BinRoot = Join-Path $TempRoot "bin"; $originalPath = $env:PATH; $children=@()
New-Item -ItemType Directory -Force -Path $CacheRoot,$BinRoot | Out-Null
try {
    Assert-Equal "check`n--manifest-path`nserver/Cargo.toml" (ConvertTo-ValidationCommand @("check","--manifest-path","server\Cargo.toml")) "command normalization"
    Assert-Equal "light" (Get-ValidationResourceClass @("check")) "check resource class"
    Assert-Equal "heavy" (Get-ValidationResourceClass @("test","filter")) "test resource class"
    $SnapshotRepo = Join-Path $TempRoot "snapshot-repo"
    New-Item -ItemType Directory -Force -Path (Join-Path $SnapshotRepo "server"),(Join-Path $SnapshotRepo "docs") | Out-Null
    & git -C $SnapshotRepo init --quiet; & git -C $SnapshotRepo config user.email validation@example.invalid; & git -C $SnapshotRepo config user.name validation-test
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "server\Cargo.lock") -Value "lock"; Set-Content -LiteralPath (Join-Path $SnapshotRepo "server\lib.rs") -Value "pub fn stable() {}"
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "docs\note.md") -Value "one"; & git -C $SnapshotRepo add .; & git -C $SnapshotRepo commit -m baseline --quiet
    $baseDetails = Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")
    $baseFingerprint = $baseDetails.fingerprint
    Assert-True ($baseDetails.payload.project -like 'no-origin:*') "project without origin must use safe hashed fallback"
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "docs\adjacent.md") -Value "unrelated"
    Assert-Equal $baseFingerprint (Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")).fingerprint "unrelated docs must not invalidate Rust evidence"
    Set-Content -LiteralPath (Join-Path $SnapshotRepo 'server\lib.rs') 'pub fn staged() {}'
    $unstagedFingerprint=(Get-ValidationFingerprint $SnapshotRepo @('check')).fingerprint; & git -C $SnapshotRepo add server/lib.rs
    Assert-Equal $unstagedFingerprint (Get-ValidationFingerprint $SnapshotRepo @('check')).fingerprint "staging must not change an exact content fingerprint"
    & git -C $SnapshotRepo reset --hard HEAD --quiet
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "server\new.rs") -Value "pub fn changed() {}"
    Assert-True ($baseFingerprint -ne (Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")).fingerprint) "relevant untracked Rust must invalidate evidence"
    Remove-Item (Join-Path $SnapshotRepo 'server\new.rs'); New-Item -ItemType Directory -Force (Join-Path $SnapshotRepo '.cargo')|Out-Null
    Set-Content (Join-Path $SnapshotRepo '.cargo\config.toml') '[build]'; Assert-True ($baseFingerprint -ne (Get-ValidationFingerprint $SnapshotRepo @('check')).fingerprint) ".cargo config must invalidate evidence"
    Remove-Item (Join-Path $SnapshotRepo '.cargo') -Recurse -Force
    $oldRustflags=$env:RUSTFLAGS; try { $env:RUSTFLAGS='--cfg elon_secret_value'; $envDetails=Get-ValidationFingerprint $SnapshotRepo @('check'); Assert-True ($envDetails.fingerprint -ne $baseFingerprint) "compile environment must invalidate evidence"; Assert-True (($envDetails.payload.environment_hashes.RUSTFLAGS -ne $env:RUSTFLAGS) -and (($envDetails.payload|ConvertTo-Json -Depth 8) -notmatch 'elon_secret_value')) "raw environment value must not persist" } finally { if($null -eq $oldRustflags){Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue}else{$env:RUSTFLAGS=$oldRustflags} }
    $stale = Join-Path $CacheRoot "stale.lock"; New-Item -ItemType Directory -Force -Path $stale | Out-Null
    '{"pid":2147483000}' | Set-Content -LiteralPath (Join-Path $stale "owner.json") -Encoding UTF8
    $lease = Enter-ValidationLock -LockPath $stale -Kind "crash-recovery" -TimeoutSeconds 2
    Assert-Equal $PID ([int]$lease.owner.pid) "stale lock should be recovered"; Exit-ValidationLock $lease
    $reusedPid=Join-Path $CacheRoot 'reused-pid.lock'; New-Item -ItemType Directory -Force $reusedPid|Out-Null
    [ordered]@{lease_id='stale';pid=$PID;process_start_id='wrong-start'}|ConvertTo-Json|Set-Content (Join-Path $reusedPid 'owner.json')
    $fresh=Enter-ValidationLock $reusedPid 'pid-reuse' 2
    Assert-True ($fresh.owner.lease_id -ne 'stale') "PID reuse must not preserve a stale lease"
    $oldLease=$fresh; Exit-ValidationLock $fresh; $successor=Enter-ValidationLock $reusedPid 'successor' 2
    Exit-ValidationLock $oldLease
    Assert-Equal $successor.owner.lease_id (Get-ValidationOwner $reusedPid).lease_id "old owner must not delete successor lock"
    Exit-ValidationLock $successor
    $resourceRoot=Join-Path $CacheRoot 'resource-contract'
    $lightA=Enter-ValidationResource $resourceRoot 'light' 2 2; $lightB=Enter-ValidationResource $resourceRoot 'light' 2 2
    Assert-Equal 2 @($lightA,$lightB).Count "two light tasks may overlap"
    $heavyBlocked=$false; try { Enter-ValidationResource $resourceRoot 'heavy' 2 0 | Out-Null } catch {$heavyBlocked=$true}
    Assert-True $heavyBlocked "heavy must exclude active light tasks"
    Exit-ValidationResource $lightA; Exit-ValidationResource $lightB
    $heavy=Enter-ValidationResource $resourceRoot 'heavy' 2 2
    $lightBlocked=$false; try { Enter-ValidationResource $resourceRoot 'light' 2 0 | Out-Null } catch {$lightBlocked=$true}
    Assert-True $lightBlocked "light must exclude an active heavy task"; Exit-ValidationResource $heavy
    $capture = Invoke-ValidationCapturedProcess -FilePath "cmd.exe" -ArgumentList @("/d","/c","echo failure-marker 1>&2 & exit /b 23") -WorkingDirectory $RepoRoot -EvidenceDirectory (Join-Path $CacheRoot "capture")
    Assert-Equal 23 $capture.exit_code "captured exit code"; Assert-True (Test-Path $capture.stderr_path) "stderr must be durable on first run"
    $Counter = Join-Path $TempRoot "counter.log"
    $readyName='Local\ElonValidationReady'+[Guid]::NewGuid().ToString('N'); $releaseName='Local\ElonValidationRelease'+[Guid]::NewGuid().ToString('N')
    $ready=New-Object Threading.EventWaitHandle($false,[Threading.EventResetMode]::ManualReset,$readyName)
    $release=New-Object Threading.EventWaitHandle($false,[Threading.EventResetMode]::ManualReset,$releaseName)
    $env:ELON_TEST_READY_EVENT=$readyName; $env:ELON_TEST_RELEASE_EVENT=$releaseName; $env:ELON_TEST_COUNTER=$Counter
    '@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fake-cargo.ps1" %*
exit /b %ERRORLEVEL%' | Set-Content -LiteralPath (Join-Path $BinRoot "cargo.cmd") -Encoding ASCII
    '$ready=[Threading.EventWaitHandle]::OpenExisting($env:ELON_TEST_READY_EVENT)
$release=[Threading.EventWaitHandle]::OpenExisting($env:ELON_TEST_RELEASE_EVENT)
Add-Content -LiteralPath $env:ELON_TEST_COUNTER -Value run
$ready.Set() | Out-Null
if(-not $release.WaitOne([TimeSpan]::FromSeconds(15))){exit 91}
exit 0' | Set-Content -LiteralPath (Join-Path $BinRoot "fake-cargo.ps1") -Encoding UTF8
    $env:PATH = "$BinRoot;$originalPath"
    $validator = Join-Path $PSScriptRoot "validate-rust.ps1"
    $stdout1 = Join-Path $TempRoot "one.out"; $stderr1 = Join-Path $TempRoot "one.err"; $stdout2 = Join-Path $TempRoot "two.out"; $stderr2 = Join-Path $TempRoot "two.err"
    $common = @("-NoProfile","-ExecutionPolicy","Bypass","-File",$validator,"-CacheRoot",$CacheRoot,"-SkipCheapGates","-DisableSccache","check","--manifest-path","server\Cargo.toml")
    $p1 = Start-Process powershell -ArgumentList $common -PassThru -NoNewWindow -RedirectStandardOutput $stdout1 -RedirectStandardError $stderr1
    $children += $p1; $null = $p1.Handle
    if(-not $ready.WaitOne([TimeSpan]::FromSeconds(30))){ throw "first validation did not reach cargo: exited=$($p1.HasExited) stdout=$((Get-Content -Raw $stdout1 -ErrorAction SilentlyContinue)) stderr=$((Get-Content -Raw $stderr1 -ErrorAction SilentlyContinue))" }
    Assert-True $true "first validation reached explicit cargo synchronization point"
    $p2 = Start-Process powershell -ArgumentList $common -PassThru -NoNewWindow -RedirectStandardOutput $stdout2 -RedirectStandardError $stderr2
    $children += $p2; $null = $p2.Handle
    $waiterPattern=Join-Path $CacheRoot 'validation-v1\evidence\*\.run.lock.waiters\*.json'
    $waiterDeadline=[DateTime]::UtcNow.AddSeconds(15)
    while(-not (Get-ChildItem $waiterPattern -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $waiterDeadline){ [Threading.Thread]::Yield() | Out-Null }
    Assert-True ($null -ne (Get-ChildItem $waiterPattern -ErrorAction SilentlyContinue)) "waiter state must be durably visible before owner release"
    $release.Set() | Out-Null
    $p1.WaitForExit(); $p2.WaitForExit(); $p1.Refresh(); $p2.Refresh()
    if ($p1.ExitCode -ne 0 -or $p2.ExitCode -ne 0) {
        throw "validator subprocess failed: one=$($p1.ExitCode) two=$($p2.ExitCode)`none=$((Get-Content -Raw $stderr1))`ntwo=$((Get-Content -Raw $stderr2))"
    }
    Assert-Equal 0 $p1.ExitCode "first exact validation"; Assert-Equal 0 $p2.ExitCode "coalesced exact validation"
    Assert-Equal 1 @(Get-Content $Counter).Count "same in-flight fingerprint must launch once"
    Assert-True ((Get-Content -Raw $stdout2) -match 'VALIDATION_REUSED=coalesced_wait') "waiter must report coalesced_wait, not completed reuse"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $validator -CacheRoot $CacheRoot -SkipCheapGates -DisableSccache check --manifest-path server\Cargo.toml | Out-Null
    Assert-Equal 1 @(Get-Content $Counter).Count "successful exact fingerprint must be reused"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $validator -CacheRoot $CacheRoot -SkipCheapGates -DisableSccache check --manifest-path server\Cargo.toml --features distinct | Out-Null
    Assert-Equal 2 @(Get-Content $Counter).Count "different fingerprint must not be reused"
    Write-Host "PASS: validation orchestrator tests ($script:Assertions assertions)." -ForegroundColor Green
} finally {
    @($children) | ForEach-Object { Stop-TestProcess $_ }
    if($ready){$ready.Dispose()}; if($release){$release.Set()|Out-Null;$release.Dispose()}
    $env:PATH=$originalPath; Remove-Item Env:ELON_TEST_READY_EVENT,Env:ELON_TEST_RELEASE_EVENT,Env:ELON_TEST_COUNTER -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    $left=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {$_.CommandLine -like "*$TempRoot*"})
    Assert-Equal 0 $left.Count "no elon-validation-test child process may remain"
}

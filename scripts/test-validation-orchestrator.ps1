$ErrorActionPreference = "Stop"
$RepoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$Modules = Join-Path $PSScriptRoot "validation"
Import-Module (Join-Path $Modules "Validation.Fingerprint.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Scheduler.psm1") -Force -DisableNameChecking
Import-Module (Join-Path $Modules "Validation.Evidence.psm1") -Force -DisableNameChecking
$script:Assertions = 0
function Assert-True([bool]$Condition,[string]$Message) { $script:Assertions++; if (-not $Condition) { throw "ASSERT FAILED: $Message" } }
function Assert-Equal($Expected,$Actual,[string]$Message) { $script:Assertions++; if ($Expected -ne $Actual) { throw "ASSERT FAILED: $Message expected=$Expected actual=$Actual" } }
$TempRoot = Join-Path $env:TEMP ("elon-validation-test-" + [Guid]::NewGuid().ToString("N"))
$CacheRoot = Join-Path $TempRoot "cache"; $BinRoot = Join-Path $TempRoot "bin"; $originalPath = $env:PATH
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
    $baseFingerprint = (Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")).fingerprint
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "docs\adjacent.md") -Value "unrelated"
    Assert-Equal $baseFingerprint (Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")).fingerprint "unrelated docs must not invalidate Rust evidence"
    Set-Content -LiteralPath (Join-Path $SnapshotRepo "server\new.rs") -Value "pub fn changed() {}"
    Assert-True ($baseFingerprint -ne (Get-ValidationFingerprint -RepoRoot $SnapshotRepo -CargoArgs @("check")).fingerprint) "relevant untracked Rust must invalidate evidence"
    $stale = Join-Path $CacheRoot "stale.lock"; New-Item -ItemType Directory -Force -Path $stale | Out-Null
    '{"pid":2147483000}' | Set-Content -LiteralPath (Join-Path $stale "owner.json") -Encoding UTF8
    $lease = Enter-ValidationLock -LockPath $stale -Kind "crash-recovery" -TimeoutSeconds 2
    Assert-Equal $PID ([int]$lease.owner.pid) "stale lock should be recovered"; Exit-ValidationLock $lease
    $capture = Invoke-ValidationCapturedProcess -FilePath "cmd.exe" -ArgumentList @("/d","/c","echo failure-marker 1>&2 & exit /b 23") -WorkingDirectory $RepoRoot -EvidenceDirectory (Join-Path $CacheRoot "capture")
    Assert-Equal 23 $capture.exit_code "captured exit code"; Assert-True (Test-Path $capture.stderr_path) "stderr must be durable on first run"
    $Counter = Join-Path $TempRoot "counter.log"
    "@echo off`necho run>>`"$Counter`"`npowershell -NoProfile -Command `"Start-Sleep -Milliseconds 1500`"`nexit /b 0" | Set-Content -LiteralPath (Join-Path $BinRoot "cargo.cmd") -Encoding ASCII
    $env:PATH = "$BinRoot;$originalPath"
    $validator = Join-Path $PSScriptRoot "validate-rust.ps1"
    $stdout1 = Join-Path $TempRoot "one.out"; $stderr1 = Join-Path $TempRoot "one.err"; $stdout2 = Join-Path $TempRoot "two.out"; $stderr2 = Join-Path $TempRoot "two.err"
    $common = @("-NoProfile","-ExecutionPolicy","Bypass","-File",$validator,"-CacheRoot",$CacheRoot,"-SkipCheapGates","-DisableSccache","check","--manifest-path","server\Cargo.toml")
    $p1 = Start-Process powershell -ArgumentList $common -PassThru -NoNewWindow -RedirectStandardOutput $stdout1 -RedirectStandardError $stderr1
    $null = $p1.Handle
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while (-not (Test-Path $Counter) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
    Assert-True (Test-Path $Counter) "first validation must reach explicit cargo synchronization point"
    $p2 = Start-Process powershell -ArgumentList $common -PassThru -NoNewWindow -RedirectStandardOutput $stdout2 -RedirectStandardError $stderr2
    $null = $p2.Handle
    $p1.WaitForExit(); $p2.WaitForExit(); $p1.Refresh(); $p2.Refresh()
    if ($p1.ExitCode -ne 0 -or $p2.ExitCode -ne 0) {
        throw "validator subprocess failed: one=$($p1.ExitCode) two=$($p2.ExitCode)`none=$((Get-Content -Raw $stderr1))`ntwo=$((Get-Content -Raw $stderr2))"
    }
    Assert-Equal 0 $p1.ExitCode "first exact validation"; Assert-Equal 0 $p2.ExitCode "coalesced exact validation"
    Assert-Equal 1 @(Get-Content $Counter).Count "same in-flight fingerprint must launch once"
    Assert-True ((Get-Content -Raw $stdout2) -match 'VALIDATION_REUSED=coalesced_wait|VALIDATION_REUSED=true') "waiter must report reuse"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $validator -CacheRoot $CacheRoot -SkipCheapGates -DisableSccache check --manifest-path server\Cargo.toml | Out-Null
    Assert-Equal 1 @(Get-Content $Counter).Count "successful exact fingerprint must be reused"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $validator -CacheRoot $CacheRoot -SkipCheapGates -DisableSccache check --manifest-path server\Cargo.toml --features distinct | Out-Null
    Assert-Equal 2 @(Get-Content $Counter).Count "different fingerprint must not be reused"
    Write-Host "PASS: validation orchestrator tests ($script:Assertions assertions)." -ForegroundColor Green
} finally { $env:PATH = $originalPath; Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue }

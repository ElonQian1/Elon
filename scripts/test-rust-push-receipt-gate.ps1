[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Invoke-Captured([string]$FilePath, [string[]]$Arguments) {
    $output = & $FilePath @Arguments 2>&1 | Out-String
    [pscustomobject]@{ Code = $LASTEXITCODE; Output = $output }
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("elon-rust-push-gate-" + [guid]::NewGuid().ToString('N'))
$bashPath = @(
    (Join-Path $env:ProgramFiles 'Git\bin\bash.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Git\bin\bash.exe'),
    (Join-Path $env:LocalAppData 'Programs\Git\bin\bash.exe')
) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) } | Select-Object -First 1
if (-not $bashPath) { throw 'Git for Windows bash is required for the pre-push fixture.' }
$oldPath = $env:PATH
$oldGate = $env:ELON_ENABLE_RUST_PUSH_RECEIPT
try {
    $pushRoot = Join-Path $tempRoot 'push-fixture'
    $pushScripts = Join-Path $pushRoot 'scripts'
    $pushBin = Join-Path $pushRoot 'bin'
    New-Item -ItemType Directory -Force -Path $pushScripts, $pushBin | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'scripts\push.ps1') -Destination $pushScripts
    Set-Content -LiteralPath (Join-Path $pushScripts 'prepare-push.ps1') -Encoding UTF8 -Value @'
Write-Host 'TEST_PREPARE_PUSH_CALLED'
exit 0
'@
    Set-Content -LiteralPath (Join-Path $pushBin 'git.cmd') -Encoding ASCII -Value @'
@echo off
echo TEST_GIT_PUSH_CALLED
exit /b 0
'@
    $env:PATH = "$pushBin;$oldPath"

    Remove-Item Env:\ELON_ENABLE_RUST_PUSH_RECEIPT -ErrorAction SilentlyContinue
    $disabledPush = Invoke-Captured powershell @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $pushScripts 'push.ps1'))
    Assert-True ($disabledPush.Code -eq 0) "default push failed: $($disabledPush.Output)"
    Assert-True ($disabledPush.Output -match 'RUST_PUSH_RECEIPT_GATE=disabled') 'default push did not report disabled'
    Assert-True ($disabledPush.Output -notmatch 'TEST_PREPARE_PUSH_CALLED') 'default push called prepare-push'

    $env:ELON_ENABLE_RUST_PUSH_RECEIPT = '1'
    $enabledPush = Invoke-Captured powershell @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $pushScripts 'push.ps1'))
    Assert-True ($enabledPush.Code -eq 0) "enabled push failed: $($enabledPush.Output)"
    Assert-True ($enabledPush.Output -match 'RUST_PUSH_RECEIPT_GATE=enabled') 'enabled push did not report enabled'
    Assert-True ($enabledPush.Output -match 'TEST_PREPARE_PUSH_CALLED') 'enabled push did not call prepare-push'

    $hookRoot = Join-Path $tempRoot 'hook-fixture'
    $hookDir = Join-Path $hookRoot '.githooks'
    $hookScripts = Join-Path $hookRoot 'scripts'
    New-Item -ItemType Directory -Force -Path $hookDir, $hookScripts, (Join-Path $hookRoot 'server') | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot '.githooks\pre-push') -Destination $hookDir
    Set-Content -LiteralPath (Join-Path $hookRoot 'server\Cargo.toml') -Encoding ASCII -Value '[package]'
    [IO.File]::WriteAllText((Join-Path $hookScripts 'format-rust.sh'), "#!/usr/bin/env bash`nexit 0`n")
    Set-Content -LiteralPath (Join-Path $hookScripts 'check-source-size.ps1') -Encoding UTF8 -Value 'exit 0'
    Set-Content -LiteralPath (Join-Path $hookScripts 'prepare-push.ps1') -Encoding UTF8 -Value @'
Write-Host 'TEST_PREPARE_PUSH_CALLED'
exit 0
'@
    $env:PATH = $oldPath
    & git -C $hookRoot init --quiet
    if ($LASTEXITCODE -ne 0) { throw 'failed to initialize Git hook fixture' }

    Remove-Item Env:\ELON_ENABLE_RUST_PUSH_RECEIPT -ErrorAction SilentlyContinue
    $disabledHook = Invoke-Captured $bashPath @((Join-Path $hookDir 'pre-push'))
    Assert-True ($disabledHook.Code -eq 0) "default pre-push failed: $($disabledHook.Output)"
    Assert-True ($disabledHook.Output -match 'RUST_PUSH_RECEIPT_GATE=disabled') 'default pre-push did not report disabled'
    Assert-True ($disabledHook.Output -notmatch 'TEST_PREPARE_PUSH_CALLED') 'default pre-push called prepare-push'

    $env:ELON_ENABLE_RUST_PUSH_RECEIPT = '1'
    $enabledHook = Invoke-Captured $bashPath @((Join-Path $hookDir 'pre-push'))
    Assert-True ($enabledHook.Code -eq 0) "enabled pre-push failed: $($enabledHook.Output)"
    Assert-True ($enabledHook.Output -match 'RUST_PUSH_RECEIPT_GATE=enabled') 'enabled pre-push did not report enabled'
    Assert-True ($enabledHook.Output -match 'TEST_PREPARE_PUSH_CALLED') 'enabled pre-push did not call prepare-push'

    Write-Host 'PASS: Rust push receipt gate defaults off and explicit opt-in enables it'
}
finally {
    $env:PATH = $oldPath
    if ($null -eq $oldGate) {
        Remove-Item Env:\ELON_ENABLE_RUST_PUSH_RECEIPT -ErrorAction SilentlyContinue
    } else {
        $env:ELON_ENABLE_RUST_PUSH_RECEIPT = $oldGate
    }
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}

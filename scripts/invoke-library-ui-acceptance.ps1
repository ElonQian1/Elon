#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','features','library','browse','query','file','attach','remove_staged','close_detail','back')]
    [string]$Step = 'inspect',
    [string]$Handle = '',
    [string]$SdkRoot = 'D:/Android/sdk',
    [string]$JavaHome = 'C:/Program Files/Microsoft/jdk-21.0.11.10-hotspot'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb (Join-Path $SdkRoot 'platform-tools/adb.exe') `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
if ($Step -eq 'file' -and $Handle -notmatch '^[a-zA-Z0-9_-]{1,160}$') { throw 'Invalid library handle.' }
$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $PSScriptRoot 'android/LibraryUiAcceptance.java'
$platform = Join-Path $SdkRoot 'platforms/android-35'
$r8 = Join-Path $SdkRoot 'build-tools/35.0.0/lib/d8.jar'
$classpath = @('android.jar','uiautomator.jar','optional/android.test.base.jar') |
    ForEach-Object { Join-Path $platform $_ }
$inputs = @($source, $r8) + $classpath
$hash = ($inputs | ForEach-Object { (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash }) -join ''
$key = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($hash))).Substring(0,16)
$output = Join-Path $root ".ai-tmp/library-ui-$key"
$dex = Join-Path $output 'library-ui.jar'
function Run-Native([string]$File, [string[]]$Arguments, [string]$Label) {
    $result = Invoke-ElonNativeCommand -FilePath $File -ArgumentList $Arguments `
        -WorkingDirectory $root -TimeoutSeconds 50 -Label $Label
    Assert-ElonNativeCommand -Result $result -FailureMessage $Label
    return $result
}
if (-not (Test-Path -LiteralPath $dex)) {
    $classes = Join-Path $output 'classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    Run-Native (Join-Path $JavaHome 'bin/javac.exe') @('-encoding','UTF-8','-source','8','-target','8',
        '-classpath',($classpath -join ';'),'-d',$classes,$source) 'compile semantic UI acceptance' | Out-Null
    $jar = Join-Path $output 'classes.jar'
    Run-Native (Join-Path $JavaHome 'bin/jar.exe') @('cf',$jar,'-C',$classes,'.') 'pack UI acceptance' | Out-Null
    Run-Native (Join-Path $JavaHome 'bin/java.exe') @('-cp',$r8,'com.android.tools.r8.D8',
        '--min-api','26','--lib',$classpath[0],'--output',$dex,$jar) 'dex UI acceptance' | Out-Null
}
$remote = "/data/local/tmp/elon-library-ui-$([Guid]::NewGuid().ToString('N')).jar"
try {
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments @('push',$dex,$remote) `
        -TimeoutSec 10 -Label 'push external semantic UI test' | Out-Null
    $arguments = @('shell','uiautomator','runtest',$remote,'-s','-c','com.elon.acceptance.LibraryUiAcceptance',
        '-e','step',$Step)
    if ($Handle) { $arguments += @('-e','handle',$Handle) }
    $raw = Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments $arguments `
        -TimeoutSec 40 -Label 'run semantic library UI action'
    # Android's legacy runner can exit zero for assertion failures.
    if ($raw -notmatch 'OK \(1 test\)' -or $raw -match 'FAILURES!!!|INSTRUMENTATION_FAILED|run aborted|shortMsg=') {
        $code = [regex]::Match($raw, '(?m)(?:AssertionFailedError|AssertionError): ([a-z_]+)').Groups[1].Value
        throw "Semantic library UI acceptance failed: $code"
    }
    $match = [regex]::Match($raw, 'LIBRARY_UI_RESULT=(\{[^\r\n]+\})')
    if (-not $match.Success) { throw 'Missing semantic UI result.' }
    $match.Groups[1].Value | ConvertFrom-Json | ConvertTo-Json -Compress
} finally {
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments @('shell','rm','-f',$remote) `
        -TimeoutSec 5 -Label 'remove external UI test' | Out-Null
}

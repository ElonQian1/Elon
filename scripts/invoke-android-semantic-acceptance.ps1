#requires -Version 7.0

function Invoke-AndroidSemanticAcceptance {
    param(
        [Parameter(Mandatory)]$Runtime,
        [Parameter(Mandatory)][ValidateSet('LibraryUiAcceptance','ConversationUiAcceptance')][string]$TestClass,
        [Parameter(Mandatory)][string]$Step,
        [hashtable]$Parameters = @{},
        [Parameter(Mandatory)][ValidateSet('LIBRARY_UI_RESULT','CONVERSATION_UI_RESULT')][string]$ResultPrefix,
        [string]$SdkRoot = 'D:/Android/sdk',
        [string]$JavaHome = 'C:/Program Files/Microsoft/jdk-21.0.11.10-hotspot'
    )
    $root = Split-Path -Parent $PSScriptRoot
    $source = Join-Path $PSScriptRoot "android/$TestClass.java"
    $platform = Join-Path $SdkRoot 'platforms/android-35'
    $r8 = Join-Path $SdkRoot 'build-tools/35.0.0/lib/d8.jar'
    $classpath = @('android.jar','uiautomator.jar','optional/android.test.base.jar') |
        ForEach-Object { Join-Path $platform $_ }
    $hash = (@($source, $r8) + $classpath | ForEach-Object {
        (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash
    }) -join ''
    $key = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData(
        [Text.Encoding]::UTF8.GetBytes($hash))).Substring(0,16)
    $output = Join-Path $root ".ai-tmp/semantic-ui-$key"
    $dex = Join-Path $output 'semantic-ui.jar'
    function Run-Native([string]$File, [string[]]$Arguments, [string]$Label) {
        $result = Invoke-ElonNativeCommand -FilePath $File -ArgumentList $Arguments `
            -WorkingDirectory $root -TimeoutSeconds 50 -Label $Label
        Assert-ElonNativeCommand -Result $result -FailureMessage $Label
    }
    if (-not (Test-Path -LiteralPath $dex)) {
        $classes = Join-Path $output 'classes'
        New-Item -ItemType Directory -Force -Path $classes | Out-Null
        Run-Native (Join-Path $JavaHome 'bin/javac.exe') @('-encoding','UTF-8','-source','8','-target','8',
            '-classpath',($classpath -join ';'),'-d',$classes,$source) 'compile semantic UI acceptance'
        $jar = Join-Path $output 'classes.jar'
        Run-Native (Join-Path $JavaHome 'bin/jar.exe') @('cf',$jar,'-C',$classes,'.') 'pack UI acceptance'
        Run-Native (Join-Path $JavaHome 'bin/java.exe') @('-cp',$r8,'com.android.tools.r8.D8',
            '--min-api','26','--lib',$classpath[0],'--output',$dex,$jar) 'dex UI acceptance'
    }
    $remote = "/data/local/tmp/elon-semantic-ui-$([Guid]::NewGuid().ToString('N')).jar"
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('push',$dex,$remote) `
            -TimeoutSec 10 -Label 'push external semantic UI test' | Out-Null
        $arguments = @('shell','uiautomator','runtest',$remote,'-s','-c',"com.elon.acceptance.$TestClass",'-e','step',$Step)
        foreach ($entry in $Parameters.GetEnumerator()) { $arguments += @('-e',[string]$entry.Key,[string]$entry.Value) }
        $raw = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments $arguments `
            -TimeoutSec 40 -Label 'run semantic UI action'
        if ($raw -notmatch 'OK \(1 test\)' -or $raw -match 'FAILURES!!!|INSTRUMENTATION_FAILED|run aborted|shortMsg=') {
            $code = [regex]::Match($raw, '(?m)(?:AssertionFailedError|AssertionError|ComparisonFailure): ([a-z_]+)').Groups[1].Value
            if (-not $code) { $code = [regex]::Match($raw, '(?m)\b([A-Za-z][A-Za-z0-9_.]*(?:Exception|Error))\b').Groups[1].Value }
            if (-not $code) { $code = 'runner_failed_without_assertion' }
            throw "Semantic UI acceptance failed: $code"
        }
        $match = [regex]::Match($raw, [regex]::Escape($ResultPrefix) + '=(\{[^\r\n]+\})')
        if (-not $match.Success) { throw 'Missing semantic UI result.' }
        return $match.Groups[1].Value | ConvertFrom-Json
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('shell','rm','-f',$remote) `
            -TimeoutSec 5 -Label 'remove external UI test' | Out-Null
    }
}

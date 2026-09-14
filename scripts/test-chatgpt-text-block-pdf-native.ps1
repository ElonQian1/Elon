#requires -Version 7.0
param(
    [string]$DeviceSerial = 'e0d909c3',
    [string]$SdkRoot = 'D:/Android/sdk',
    [string]$JavaHome = $env:JAVA_HOME,
    [string]$GradleHome = (Join-Path $HOME '.gradle'),
    [switch]$CompileOnly
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'native-command-timeout.ps1')
$root = Split-Path -Parent $PSScriptRoot
$id = [Guid]::NewGuid().ToString('N')
$output = Join-Path $root ".ai-tmp/pdf-native-$id"
$classes = Join-Path $root 'android/app/build/tmp/kotlin-classes/release'
$platform = Join-Path $SdkRoot 'platforms/android-34/android.jar'
$d8 = Join-Path $SdkRoot 'build-tools/34.0.0/lib/d8.jar'
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
if (-not $JavaHome) { $JavaHome = Split-Path -Parent (Split-Path -Parent (Get-Command java.exe).Source) }
function Run([string]$File, [string[]]$Arguments, [string]$Label, [int]$Timeout = 60) {
    $result = Invoke-ElonNativeCommand -FilePath $File -ArgumentList $Arguments `
        -WorkingDirectory $root -TimeoutSeconds $Timeout -Label $Label
    Assert-ElonNativeCommand -Result $result -FailureMessage $Label
    return [string]$result.Stdout
}
function Jar([string]$Group, [string]$Module, [string]$Version) {
    $directory = Join-Path $GradleHome "caches/modules-2/files-2.1/$Group/$Module/$Version"
    $found = @(Get-ChildItem -LiteralPath $directory -Recurse -File -Filter "$Module-$Version.jar")
    if ($found.Count -ne 1) { throw "dependency_not_resolved:$Module" }
    return $found[0].FullName
}
$dependencies = @(
    (Jar 'org.jetbrains.kotlin' 'kotlin-stdlib' '1.9.22'),
    (Jar 'com.atlassian.commonmark' 'commonmark' '0.13.0'),
    (Jar 'com.atlassian.commonmark' 'commonmark-ext-gfm-tables' '0.13.0'),
    (Jar 'com.atlassian.commonmark' 'commonmark-ext-gfm-strikethrough' '0.13.0')
)
$selected = @(Get-ChildItem (Join-Path $classes 'com/elon/app') -File -Filter '*.class' | Where-Object {
    $_.Name -match '^WebChatTextBlock(Pdf.*|Export.*|\$Companion)?\.class$'
}) + @(Get-ChildItem (Join-Path $classes 'com/elon/app/chatgptweb') -File -Filter 'ChatGptWebCanvasExportFormat*.class')
foreach ($name in @('WebChatTextBlockPdf', 'WebChatTextBlockPdfContent', 'WebChatTextBlockExport')) {
    $binary = $selected | Where-Object Name -CEQ "$name.class"
    $source = Get-Item (Join-Path $root "android/app/src/main/kotlin/com/elon/app/$name.kt")
    if (-not $binary -or $binary.LastWriteTimeUtc -lt $source.LastWriteTimeUtc) { throw "compile_current_release_first:$name" }
}
New-Item -ItemType Directory -Path (Join-Path $output 'runner') -Force | Out-Null
$production = Join-Path $output 'production.jar'
$arguments = @('cf', $production)
foreach ($file in $selected) { $arguments += @('-C', $classes, [IO.Path]::GetRelativePath($classes, $file.FullName)) }
Run (Join-Path $JavaHome 'bin/jar.exe') $arguments 'pack compiled PDF implementation' | Out-Null
$classpath = (@($platform, $production) + $dependencies) -join ';'
Run (Join-Path $JavaHome 'bin/javac.exe') @('-encoding','UTF-8','-source','8','-target','8',
    '-classpath',$classpath,'-d',(Join-Path $output 'runner'),
    (Join-Path $PSScriptRoot 'android/TextBlockPdfAcceptance.java')) 'compile native PDF acceptance' | Out-Null
$runner = Join-Path $output 'runner.jar'
Run (Join-Path $JavaHome 'bin/jar.exe') @('cf',$runner,'-C',(Join-Path $output 'runner'),'.') 'pack native PDF acceptance' | Out-Null
$dex = Join-Path $output 'payload.jar'
Run (Join-Path $JavaHome 'bin/java.exe') (@('-cp',$d8,'com.android.tools.r8.D8','--min-api','26',
    '--lib',$platform,'--output',$dex,$production,$runner) + $dependencies) 'dex native PDF acceptance' 90 | Out-Null
Write-Output "PDF_NATIVE_HARNESS_COMPILED=true"
Write-Output "PDF_NATIVE_PAYLOAD_SHA256=$((Get-FileHash $dex -Algorithm SHA256).Hash.ToLowerInvariant())"
if ($CompileOnly) { Write-Output 'PDF_NATIVE_DEVICE_STATUS=not_run'; exit 0 }
if ((Run $adb @('-s',$DeviceSerial,'get-state') 'verify PDF device transport' 8).Trim() -cne 'device') { throw 'device_not_online' }
$remote = "/data/local/tmp/elon-pdf-$id"
$created = $false
try {
    Run $adb @('-s',$DeviceSerial,'shell','mkdir',$remote) 'create isolated PDF fixture directory' 8 | Out-Null
    $created = $true
    Run $adb @('-s',$DeviceSerial,'push',$dex,"$remote/payload.jar") 'push native PDF fixture' 15 | Out-Null
    $raw = Run $adb @('-s',$DeviceSerial,'shell',"CLASSPATH=$remote/payload.jar",'app_process','/',
        'com.elon.acceptance.TextBlockPdfAcceptance',$remote) 'run production PDF exporter without changing app state' 45
    $match = [regex]::Match($raw, '(?m)^PDF_NATIVE_RESULT=(\{[^\r\n]+\})')
    if (-not $match.Success) { throw 'native_pdf_receipt_missing' }
    $receipt = $match.Groups[1].Value | ConvertFrom-Json
    if ($receipt.schema -cne 'elon.writing_pdf_native.v1' -or $receipt.passed -ne $true -or $receipt.cases -ne 4) {
        throw 'native_pdf_receipt_invalid'
    }
    foreach ($name in @('sample.pdf','preview.png')) {
        Run $adb @('-s',$DeviceSerial,'pull',"$remote/$name",(Join-Path $output $name)) 'read synthetic PDF evidence' 15 | Out-Null
    }
    Write-Output $match.Value
    Write-Output "PDF_NATIVE_EVIDENCE=$output"
} finally {
    if ($remote -cnotmatch '^/data/local/tmp/elon-pdf-[a-f0-9]{32}$') { throw 'invalid_cleanup_path' }
    if ($created) {
        Run $adb @('-s',$DeviceSerial,'shell','rm','-f',"$remote/payload.jar","$remote/check.pdf","$remote/sample.pdf","$remote/preview.png") 'remove owned PDF fixtures' 8 | Out-Null
        Run $adb @('-s',$DeviceSerial,'shell','rmdir',$remote) 'remove empty PDF fixture directory' 8 | Out-Null
    }
}

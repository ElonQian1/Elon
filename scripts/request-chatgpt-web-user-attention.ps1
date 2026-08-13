#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)]
    [ValidateSet("unlock_device", "attachment", "dictation", "realtime_voice", "sensitive_action")]
    [string]$Action,
    [Parameter(Mandatory = $true)][ValidateLength(1, 120)][string]$Message,
    [switch]$SkipHaptic
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Request-ChatGptWebSmokeUserAttention -Runtime $runtime -Action $Action `
    -Message $Message -SkipHaptic:$SkipHaptic | ConvertTo-Json -Compress

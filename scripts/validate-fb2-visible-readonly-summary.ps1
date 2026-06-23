#requires -Version 7.0

param(
    [string]$SummaryPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-visible-readonly-validation.ps1")

function Get-Fb2VisibleReadonlyRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2VisibleReadonlyPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

if ($SelfTest) {
    Invoke-ReadOnlyDirectReadValidationSelfTest
    exit 0
}

$root = Get-Fb2VisibleReadonlyRepoRoot
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Join-Path $root "target\fb2-ai-center\read-only-direct-read-current.json"
} else {
    $SummaryPath = Resolve-Fb2VisibleReadonlyPath -Path $SummaryPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\visible-readonly-summary-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2VisibleReadonlyPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$summary = Read-JsonFileOrNull -Path $SummaryPath
$result = New-ReadOnlyDirectReadValidation -Summary $summary -SourcePath $SummaryPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}

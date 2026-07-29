param(
    [string]$SourcePath = "",
    [string]$RepoRoot = "",
    [switch]$Check
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
}
if ([string]::IsNullOrWhiteSpace($RepoRoot) -or -not (Test-Path -LiteralPath $RepoRoot)) {
    throw "Cannot resolve repository root."
}
$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
$canonicalRelative = "assets\brand\logo.png"
$canonicalPath = Join-Path $RepoRoot $canonicalRelative
if ([string]::IsNullOrWhiteSpace($SourcePath)) {
    $SourcePath = $canonicalPath
}
$SourcePath = [System.IO.Path]::GetFullPath($SourcePath)
if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
    throw "Brand logo source does not exist: $SourcePath"
}

function Save-BrandPng {
    param(
        [System.Drawing.Image]$Source,
        [int]$Size,
        [string]$Destination
    )

    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $Destination)) | Out-Null
    $bitmap = New-Object System.Drawing.Bitmap(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([System.Drawing.Color]::Transparent)
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
            $graphics.DrawImage($Source, 0, 0, $Size, $Size)
        } finally {
            $graphics.Dispose()
        }
        $bitmap.Save($Destination, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
}

function Save-PngBackedIcon {
    param(
        [string]$PngPath,
        [string]$Destination,
        [int]$Size
    )

    $pngBytes = [System.IO.File]::ReadAllBytes($PngPath)
    $stream = [System.IO.File]::Create($Destination)
    try {
        $writer = New-Object System.IO.BinaryWriter($stream)
        try {
            $writer.Write([uint16]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]1)
            $iconDimension = if ($Size -ge 256) { 0 } else { $Size }
            $writer.Write([byte]$iconDimension)
            $writer.Write([byte]$iconDimension)
            $writer.Write([byte]0)
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$pngBytes.Length)
            $writer.Write([uint32]22)
            $writer.Write($pngBytes)
        } finally {
            $writer.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function New-BrandAssets {
    param(
        [string]$InputPath,
        [string]$OutputRoot
    )

    $generated = New-Object System.Collections.Generic.List[string]
    $outputCanonical = Join-Path $OutputRoot $canonicalRelative
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $outputCanonical)) | Out-Null
    if ([System.IO.Path]::GetFullPath($InputPath) -ne [System.IO.Path]::GetFullPath($outputCanonical)) {
        [System.IO.File]::Copy($InputPath, $outputCanonical, $true)
    }
    $generated.Add($canonicalRelative)

    $source = [System.Drawing.Image]::FromFile($outputCanonical)
    try {
        if ($source.Width -ne $source.Height) {
            throw "Brand logo must be square. Actual: $($source.Width)x$($source.Height)"
        }
        if ($source.Width -lt 192) {
            throw "Brand logo must be at least 192x192. Actual: $($source.Width)x$($source.Height)"
        }

        $densities = [ordered]@{
            "mdpi" = 48
            "hdpi" = 72
            "xhdpi" = 96
            "xxhdpi" = 144
            "xxxhdpi" = 192
        }
        foreach ($entry in $densities.GetEnumerator()) {
            $qualifier = [string]$entry.Key
            $baseSize = [int]$entry.Value
            $targets = [ordered]@{
                "android\app\src\main\res\drawable-$qualifier\ic_app_brand.png" = $baseSize
                "android\app\src\main\res\mipmap-$qualifier\ic_launcher.png" = $baseSize
                "android\app\src\main\res\mipmap-$qualifier\ic_launcher_round.png" = $baseSize
                "android\app\src\main\res\mipmap-$qualifier\ic_launcher_foreground.png" = [int]($baseSize * 2.25)
            }
            foreach ($target in $targets.GetEnumerator()) {
                Save-BrandPng -Source $source -Size ([int]$target.Value) `
                    -Destination (Join-Path $OutputRoot ([string]$target.Key))
                $generated.Add([string]$target.Key)
            }
        }

        $desktopPng = "desktop-shell\src-tauri\icons\icon.png"
        Save-BrandPng -Source $source -Size 192 -Destination (Join-Path $OutputRoot $desktopPng)
        $generated.Add($desktopPng)

        $projectLogo = "server\src\assets\project-icons\elon-self-logo.png"
        Save-BrandPng -Source $source -Size 96 -Destination (Join-Path $OutputRoot $projectLogo)
        $generated.Add($projectLogo)
    } finally {
        $source.Dispose()
    }

    $desktopIco = "desktop-shell\src-tauri\icons\icon.ico"
    Save-PngBackedIcon -PngPath (Join-Path $OutputRoot "desktop-shell\src-tauri\icons\icon.png") `
        -Destination (Join-Path $OutputRoot $desktopIco) -Size 192
    $generated.Add($desktopIco)

    $brandBase64 = "server\src\assets\ic_app_brand.b64"
    $brandBytes = [System.IO.File]::ReadAllBytes(
        (Join-Path $OutputRoot "desktop-shell\src-tauri\icons\icon.png")
    )
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent (Join-Path $OutputRoot $brandBase64))) | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $OutputRoot $brandBase64),
        [Convert]::ToBase64String($brandBytes),
        [System.Text.Encoding]::ASCII
    )
    $generated.Add($brandBase64)
    return $generated
}

if ($Check) {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
        ("elon-brand-logo-check-" + [Guid]::NewGuid().ToString("N"))
    [System.IO.Directory]::CreateDirectory($tempRoot) | Out-Null
    try {
        $generated = @(New-BrandAssets -InputPath $SourcePath -OutputRoot $tempRoot)
        $mismatches = @($generated | Where-Object {
            $actual = Join-Path $RepoRoot $_
            $expected = Join-Path $tempRoot $_
            -not (Test-Path -LiteralPath $actual) -or
                (Get-FileHash -LiteralPath $actual -Algorithm SHA256).Hash -ne
                (Get-FileHash -LiteralPath $expected -Algorithm SHA256).Hash
        })
        if ($mismatches.Count -gt 0) {
            throw "Brand assets are stale or missing: $($mismatches -join ', ')"
        }
        Write-Host "BRAND_LOGO_STATUS=verified generated=$($generated.Count - 1) source=$canonicalRelative"
    } finally {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
    exit 0
}

$generated = @(New-BrandAssets -InputPath $SourcePath -OutputRoot $RepoRoot)
Write-Host "BRAND_LOGO_STATUS=updated generated=$($generated.Count - 1) source=$canonicalRelative"
Write-Host "NEXT=powershell -NoProfile -ExecutionPolicy Bypass -File scripts\replace-brand-logo.ps1 -Check"

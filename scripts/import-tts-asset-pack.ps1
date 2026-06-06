param(
    [string]$AssetRoot = "D:\tts-assets",
    [string]$SourceDir = "",
    [string]$FfmpegExe = "",
    [switch]$Force,
    [switch]$ValidateOnly,
    [switch]$FailOnMissing
)

$ErrorActionPreference = "Stop"

function Write-ItemStatus {
    param([string]$Name, [bool]$Ok, [string]$Detail = "")
    $flag = if ($Ok) { "OK" } else { "MISS" }
    if ($Detail) {
        Write-Output "$flag`t$Name`t$Detail"
    } else {
        Write-Output "$flag`t$Name"
    }
}

function Resolve-Ffmpeg {
    if ($FfmpegExe) {
        if (-not (Test-Path -LiteralPath $FfmpegExe)) {
            throw "FfmpegExe not found: $FfmpegExe"
        }
        return $FfmpegExe
    }
    $cmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return ""
}

function Find-SourceFile {
    param([string]$Root, [string[]]$Aliases)
    if (-not $Root -or -not (Test-Path -LiteralPath $Root)) {
        return $null
    }
    foreach ($alias in $Aliases) {
        $direct = Join-Path $Root $alias
        if (Test-Path -LiteralPath $direct) {
            return Get-Item -LiteralPath $direct
        }
        $match = Get-ChildItem -LiteralPath $Root -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ieq $alias -or $_.FullName.Replace("\", "/").EndsWith($alias.Replace("\", "/"), [StringComparison]::OrdinalIgnoreCase) } |
            Select-Object -First 1
        if ($match) { return $match }
    }
    return $null
}

function Import-Audio {
    param(
        [System.IO.FileInfo]$Source,
        [string]$Target,
        [string]$Ffmpeg
    )
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Target) | Out-Null
    if ($Source.Extension -ieq ".wav") {
        Copy-Item -LiteralPath $Source.FullName -Destination $Target -Force
        return
    }
    if (-not $Ffmpeg) {
        throw "ffmpeg is required to convert $($Source.Extension) to wav: $($Source.FullName)"
    }
    & $Ffmpeg -y -i $Source.FullName -ac 1 -ar 24000 -sample_fmt s16 $Target | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg failed with exit code $LASTEXITCODE for $($Source.FullName)"
    }
}

function Ensure-VoiceMetadata {
    param([hashtable]$Item, [string]$WavPath, [switch]$Overwrite)
    $jsonPath = [System.IO.Path]::ChangeExtension($WavPath, ".json")
    if ((Test-Path -LiteralPath $jsonPath) -and -not $Overwrite) {
        return
    }
    $transcript = ""
    if ($SourceDir -and (Test-Path -LiteralPath $SourceDir)) {
        foreach ($alias in $Item.Aliases) {
            $txtAlias = [System.IO.Path]::ChangeExtension($alias, ".txt")
            $txtFile = Find-SourceFile -Root $SourceDir -Aliases @($txtAlias)
            if ($txtFile) {
                $transcript = (Get-Content -LiteralPath $txtFile.FullName -Raw -Encoding UTF8).Trim()
                break
            }
        }
    }
    if (-not $transcript) {
        $transcript = "Replace this with the exact transcript of the authorized reference wav."
    }
    [ordered]@{
        id = $Item.Id
        label = $Item.Label
        promptText = $transcript
        requiredWav = $Item.Target
        note = "Use only authorized female reference audio. Do not use celebrity, streamer, actor, or influencer voices."
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
}

$items = @(
    @{ Kind = "voice"; Id = "female_warm"; Label = "female_warm"; Target = "voices/female_warm_neutral.wav"; Aliases = @("voices/female_warm_neutral.wav", "female_warm_neutral.wav", "female_warm.wav", "female_warm/speaker.wav", "warm.wav") },
    @{ Kind = "voice"; Id = "female_bright"; Label = "female_bright"; Target = "voices/female_bright_neutral.wav"; Aliases = @("voices/female_bright_neutral.wav", "female_bright_neutral.wav", "female_bright.wav", "female_bright/speaker.wav", "bright.wav") },
    @{ Kind = "voice"; Id = "female_mature"; Label = "female_mature"; Target = "voices/female_mature_neutral.wav"; Aliases = @("voices/female_mature_neutral.wav", "female_mature_neutral.wav", "female_mature.wav", "female_mature/speaker.wav", "mature.wav") },
    @{ Kind = "voice"; Id = "female_cool"; Label = "female_cool"; Target = "voices/female_cool_neutral.wav"; Aliases = @("voices/female_cool_neutral.wav", "female_cool_neutral.wav", "female_cool.wav", "female_cool/speaker.wav", "cool.wav") },
    @{ Kind = "voice"; Id = "female_sweet"; Label = "female_sweet"; Target = "voices/female_sweet_neutral.wav"; Aliases = @("voices/female_sweet_neutral.wav", "female_sweet_neutral.wav", "female_sweet.wav", "female_sweet/speaker.wav", "sweet.wav") },
    @{ Kind = "emotion"; Id = "female_neutral"; Target = "emotions/female_neutral.wav"; Aliases = @("emotions/female_neutral.wav", "female_neutral.wav", "neutral.wav") },
    @{ Kind = "emotion"; Id = "female_gentle_comfort"; Target = "emotions/female_gentle_comfort.wav"; Aliases = @("emotions/female_gentle_comfort.wav", "female_gentle_comfort.wav", "gentle_comfort.wav", "comfort.wav") },
    @{ Kind = "emotion"; Id = "female_crying_broken"; Target = "emotions/female_crying_broken.wav"; Aliases = @("emotions/female_crying_broken.wav", "female_crying_broken.wav", "crying_broken.wav", "crying.wav") },
    @{ Kind = "emotion"; Id = "female_happy_soft"; Target = "emotions/female_happy_soft.wav"; Aliases = @("emotions/female_happy_soft.wav", "female_happy_soft.wav", "happy_soft.wav") },
    @{ Kind = "emotion"; Id = "female_happy_excited"; Target = "emotions/female_happy_excited.wav"; Aliases = @("emotions/female_happy_excited.wav", "female_happy_excited.wav", "happy_excited.wav", "excited.wav") },
    @{ Kind = "emotion"; Id = "female_angry_repressed"; Target = "emotions/female_angry_repressed.wav"; Aliases = @("emotions/female_angry_repressed.wav", "female_angry_repressed.wav", "angry_repressed.wav") },
    @{ Kind = "emotion"; Id = "female_cool_detached"; Target = "emotions/female_cool_detached.wav"; Aliases = @("emotions/female_cool_detached.wav", "female_cool_detached.wav", "cool_detached.wav") },
    @{ Kind = "emotion"; Id = "female_shy_nervous"; Target = "emotions/female_shy_nervous.wav"; Aliases = @("emotions/female_shy_nervous.wav", "female_shy_nervous.wav", "shy_nervous.wav", "shy.wav") },
    @{ Kind = "emotion"; Id = "female_sad_low"; Target = "emotions/female_sad_low.wav"; Aliases = @("emotions/female_sad_low.wav", "female_sad_low.wav", "sad_low.wav", "sad.wav") },
    @{ Kind = "emotion"; Id = "female_surprised"; Target = "emotions/female_surprised.wav"; Aliases = @("emotions/female_surprised.wav", "female_surprised.wav", "surprised.wav") },
    @{ Kind = "emotion"; Id = "female_serious_encourage"; Target = "emotions/female_serious_encourage.wav"; Aliases = @("emotions/female_serious_encourage.wav", "female_serious_encourage.wav", "serious_encourage.wav") },
    @{ Kind = "emotion"; Id = "female_whisper"; Target = "emotions/female_whisper.wav"; Aliases = @("emotions/female_whisper.wav", "female_whisper.wav", "whisper.wav") }
)

$ffmpeg = Resolve-Ffmpeg
$missing = 0
New-Item -ItemType Directory -Force -Path $AssetRoot | Out-Null

foreach ($item in $items) {
    $target = Join-Path $AssetRoot $item.Target
    $exists = Test-Path -LiteralPath $target
    if ($ValidateOnly) {
        Write-ItemStatus $item.Kind $exists $item.Target
        if (-not $exists) { $missing++ }
        continue
    }
    if ($exists -and -not $Force) {
        Write-ItemStatus $item.Kind $true "$($item.Target) already exists"
        continue
    }
    $source = Find-SourceFile -Root $SourceDir -Aliases $item.Aliases
    if (-not $source) {
        Write-ItemStatus $item.Kind $false "$($item.Target) source not found"
        $missing++
        continue
    }
    Import-Audio -Source $source -Target $target -Ffmpeg $ffmpeg
    if ($item.Kind -eq "voice") {
        Ensure-VoiceMetadata -Item $item -WavPath $target -Overwrite:$Force
    }
    Write-ItemStatus $item.Kind $true "$($item.Target) <- $($source.FullName)"
}

if ($missing -gt 0 -and $FailOnMissing) {
    throw "$missing required TTS asset(s) are missing."
}

param(
    [string]$AssetRoot = "D:\tts-assets",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$voices = @(
    @{ Id = "female_warm"; File = "female_warm_neutral.wav"; Label = "female_warm"; PromptText = "Replace this with the exact transcript of the authorized reference wav." },
    @{ Id = "female_bright"; File = "female_bright_neutral.wav"; Label = "female_bright"; PromptText = "Replace this with the exact transcript of the authorized reference wav." },
    @{ Id = "female_mature"; File = "female_mature_neutral.wav"; Label = "female_mature"; PromptText = "Replace this with the exact transcript of the authorized reference wav." },
    @{ Id = "female_cool"; File = "female_cool_neutral.wav"; Label = "female_cool"; PromptText = "Replace this with the exact transcript of the authorized reference wav." },
    @{ Id = "female_sweet"; File = "female_sweet_neutral.wav"; Label = "female_sweet"; PromptText = "Replace this with the exact transcript of the authorized reference wav." }
)

$emotions = @(
    "female_neutral.wav",
    "female_gentle_comfort.wav",
    "female_crying_broken.wav",
    "female_happy_soft.wav",
    "female_happy_excited.wav",
    "female_angry_repressed.wav",
    "female_cool_detached.wav",
    "female_shy_nervous.wav",
    "female_sad_low.wav",
    "female_surprised.wav",
    "female_serious_encourage.wav",
    "female_whisper.wav"
)

New-Item -ItemType Directory -Force -Path (Join-Path $AssetRoot "voices") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $AssetRoot "emotions") | Out-Null

foreach ($voice in $voices) {
    $wavPath = Join-Path $AssetRoot ("voices\" + $voice.File)
    $jsonPath = [System.IO.Path]::ChangeExtension($wavPath, ".json")
    if ((Test-Path -LiteralPath $jsonPath) -and -not $Force) {
        continue
    }
    $profile = [ordered]@{
        id = $voice.Id
        label = $voice.Label
        promptText = $voice.PromptText
        requiredWav = ("voices/" + $voice.File)
        note = "Copy the authorized female reference wav to requiredWav. Do not use celebrity, streamer, actor, or influencer voices."
    }
    $profile | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
}

$readme = @"
# Elon TTS Asset Pack

Put authorized TTS reference audio here.

- voices/*.wav: five distinct female speaker reference audios.
- voices/*.json: CosyVoice promptText metadata. Keep it aligned with the wav transcript.
- emotions/*.wav: emotion reference audios from authorized material.

Required voice files:
$($voices | ForEach-Object { "- voices/$($_.File)" } | Out-String)
Required emotion files:
$($emotions | ForEach-Object { "- emotions/$_" } | Out-String)
"@
$readme | Set-Content -LiteralPath (Join-Path $AssetRoot "README.md") -Encoding UTF8

Write-Host "Asset pack skeleton created: $AssetRoot"
Write-Host "Next: copy authorized wav files into voices/ and emotions/, then run scripts\check-tts-stack.ps1 with ELON_TTS_ASSET_ROOT set."

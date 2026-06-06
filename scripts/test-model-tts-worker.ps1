param(
    [string]$WorkerUrl = "http://127.0.0.1:5011",
    [string]$OutputDir = ".runtime\tts-model-tests",
    [string]$Provider = "index_tts2",
    [string]$Text = "Hello, I am your AI assistant. I am glad to chat with you today.",
    [string]$EmotionId = "normal",
    [string]$Intensity = "normal"
)

$ErrorActionPreference = "Stop"

$voices = @(
    @{ Id = "female_warm"; Audio = "voices/female_warm_neutral.wav" },
    @{ Id = "female_bright"; Audio = "voices/female_bright_neutral.wav" },
    @{ Id = "female_mature"; Audio = "voices/female_mature_neutral.wav" },
    @{ Id = "female_cool"; Audio = "voices/female_cool_neutral.wav" },
    @{ Id = "female_sweet"; Audio = "voices/female_sweet_neutral.wav" }
)

$emotionAudio = if ($EmotionId -eq "normal") {
    "emotions/female_neutral.wav"
} else {
    "emotions/$EmotionId.wav"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$WorkerUrl = $WorkerUrl.TrimEnd("/")

Write-Host "Health:"
curl.exe -fsS "$WorkerUrl/health"
Write-Host ""

foreach ($voice in $voices) {
    $body = [ordered]@{
        provider = $Provider
        text = $Text
        originalText = $Text
        voiceId = $voice.Id
        voiceAudio = $voice.Audio
        emotionId = $EmotionId
        emotionAudio = $emotionAudio
        intensity = $Intensity
        emoAlpha = 0.45
        speed = 1.0
    } | ConvertTo-Json -Depth 4
    $output = Join-Path $OutputDir ($voice.Id + ".wav")
    $headers = Join-Path $OutputDir ($voice.Id + ".headers.txt")
    $bodyFile = Join-Path $OutputDir ($voice.Id + ".request.json")
    $body | Set-Content -LiteralPath $bodyFile -Encoding UTF8

    Write-Host ""
    Write-Host "Synthesizing $($voice.Id) -> $output"
    curl.exe -sS -D $headers -H "Content-Type: application/json" --data-binary "@$bodyFile" "$WorkerUrl/synthesize" --output $output
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed for $($voice.Id) with exit code $LASTEXITCODE"
    }
    $status = Get-Content -LiteralPath $headers -TotalCount 1
    Write-Host $status
    if ($status -notmatch " 200 ") {
        $detail = Get-Content -LiteralPath $output -Raw -ErrorAction SilentlyContinue
        Write-Host $detail
    }
}

Write-Host ""
Write-Host "Done. Compare the 5 wav files with the same text and emotion. They must sound like distinct voices, not one voice with pitch changes."

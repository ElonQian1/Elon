<# Generates the original, single-onset chat notification tone (no external samples). #>
$ErrorActionPreference = 'Stop'
$outputPath = Join-Path $PSScriptRoot '../android/app/src/main/res/raw/chat_message_soft.wav'
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($outputPath)) | Out-Null
$sampleRate = 44100
$sampleCount = [int]($sampleRate * 0.28)
$writer = [IO.BinaryWriter]::new([IO.File]::Create($outputPath))
try {
    $writer.Write([Text.Encoding]::ASCII.GetBytes('RIFF'))
    $writer.Write([int](36 + $sampleCount * 2))
    $writer.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt '))
    $writer.Write([int]16)
    $writer.Write([int16]1)
    $writer.Write([int16]1)
    $writer.Write([int]$sampleRate)
    $writer.Write([int]($sampleRate * 2))
    $writer.Write([int16]2)
    $writer.Write([int16]16)
    $writer.Write([Text.Encoding]::ASCII.GetBytes('data'))
    $writer.Write([int]($sampleCount * 2))
    for ($sampleIndex = 0; $sampleIndex -lt $sampleCount; $sampleIndex++) {
        $time = $sampleIndex / [double]$sampleRate
        $attack = [Math]::Pow([Math]::Sin([Math]::Min(1.0, $time / 0.01) * [Math]::PI / 2), 2)
        $release = [Math]::Pow([Math]::Sin([Math]::Min(1.0, (0.28 - $time) / 0.03) * [Math]::PI / 2), 2)
        $envelope = $attack * [Math]::Exp(-$time / 0.065) * $release
        $phase = 2 * [Math]::PI * 740 * $time
        $wave = ([Math]::Sin($phase) + 0.08 * [Math]::Sin(2 * $phase)) / 1.08
        $writer.Write([int16][Math]::Round(32767 * 0.32 * $envelope * $wave))
    }
} finally { $writer.Dispose() }
Write-Output 'CHAT_SOUND_GENERATED duration_ms=280 channels=1 sample_rate=44100 format=PCM16 frequency_hz=740'

package com.elon.app

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.util.Log

/** 播放服务端 Realtime WS 回传的 PCM16/24kHz/mono 音频。 */
internal class RealtimePcmPlayer {
    companion object {
        private const val TAG = "RealtimePcmPlayer"
    }

    @Volatile
    var outputEnabled: Boolean = true

    private var audioTrack: AudioTrack? = null

    fun start(): Boolean {
        if (audioTrack != null) return true
        val minBuf = AudioTrack.getMinBufferSize(
            RealtimePcmRecorder.SAMPLE_RATE_HZ,
            AudioFormat.CHANNEL_OUT_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
        )
        if (minBuf <= 0) {
            Log.w(TAG, "AudioTrack.getMinBufferSize failed: $minBuf")
            return false
        }
        val bufferBytes = maxOf(minBuf, RealtimePcmRecorder.SAMPLE_RATE_HZ / 5 * 2)
        val track = try {
            AudioTrack(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build(),
                AudioFormat.Builder()
                    .setSampleRate(RealtimePcmRecorder.SAMPLE_RATE_HZ)
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build(),
                bufferBytes,
                AudioTrack.MODE_STREAM,
                AudioManager.AUDIO_SESSION_ID_GENERATE,
            )
        } catch (t: Throwable) {
            Log.w(TAG, "create AudioTrack failed", t)
            return false
        }
        if (track.state != AudioTrack.STATE_INITIALIZED) {
            track.release()
            Log.w(TAG, "AudioTrack not initialized: ${track.state}")
            return false
        }
        audioTrack = track
        track.play()
        return true
    }

    fun play(chunk: ByteArray) {
        if (!outputEnabled || chunk.isEmpty()) return
        val track = audioTrack ?: return
        val written = track.write(chunk, 0, chunk.size)
        if (written < 0) Log.w(TAG, "AudioTrack.write failed: $written")
    }

    fun clear() {
        val track = audioTrack ?: return
        runCatching { track.pause() }
        runCatching { track.flush() }
        runCatching { track.play() }
    }

    fun release() {
        val track = audioTrack
        audioTrack = null
        runCatching { track?.pause() }
        runCatching { track?.flush() }
        runCatching { track?.release() }
    }
}

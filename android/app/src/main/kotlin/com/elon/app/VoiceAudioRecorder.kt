package com.elon.app

import android.content.Context
import android.media.MediaRecorder
import android.os.SystemClock
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

internal class VoiceAudioRecorder(private val context: Context) {
    private var recorder: MediaRecorder? = null
    private var outputFile: File? = null
    private var startedAt: Long = 0L

    val isRecording: Boolean
        get() = recorder != null

    fun start(): Boolean {
        cancel()
        val file = voiceAttachmentFile(context)
        val created = MediaRecorder().apply {
            setAudioSource(MediaRecorder.AudioSource.MIC)
            setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
            setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
            setAudioEncodingBitRate(64_000)
            setAudioSamplingRate(44_100)
            setOutputFile(file.absolutePath)
        }
        return runCatching {
            created.prepare()
            created.start()
            recorder = created
            outputFile = file
            startedAt = SystemClock.elapsedRealtime()
        }.onFailure {
            runCatching { created.release() }
            file.delete()
        }.isSuccess
    }

    /** 录制中返回已录秒数（最少 1 秒），未录音返回 0。 */
    fun recordedDurationSeconds(): Int {
        if (recorder == null) return 0
        return ((SystemClock.elapsedRealtime() - startedAt) / 1000L).toInt().coerceAtLeast(1)
    }

    fun stopToAttachment(): PendingAttachment? {
        val durationSec = recordedDurationSeconds()
        val activeRecorder = recorder ?: return null
        val file = outputFile
        recorder = null
        outputFile = null
        startedAt = 0L
        val stopped = runCatching { activeRecorder.stop() }.isSuccess
        runCatching { activeRecorder.release() }
        if (!stopped || file == null || !file.isFile || file.length() <= MIN_VOICE_BYTES) {
            file?.delete()
            return null
        }
        val displayName = "语音_${VOICE_DISPLAY_TIME.format(Date())}.m4a"
        return PendingAttachment(
            kind = "audio",
            displayLabel = "语音",
            displayName = displayName,
            fileName = file.name,
            mimeType = "audio/mp4",
            file = file,
            durationSeconds = durationSec
        )
    }

    fun cancel() {
        val activeRecorder = recorder
        val file = outputFile
        recorder = null
        outputFile = null
        if (activeRecorder != null) {
            runCatching { activeRecorder.stop() }
            runCatching { activeRecorder.release() }
        }
        file?.delete()
    }
}

private fun voiceAttachmentFile(context: Context): File {
    val dir = File(context.cacheDir, "voice_attachments").apply { mkdirs() }
    return File(dir, "voice_${System.currentTimeMillis()}.m4a")
}

private const val MIN_VOICE_BYTES = 256L
private val VOICE_DISPLAY_TIME = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.CHINA)

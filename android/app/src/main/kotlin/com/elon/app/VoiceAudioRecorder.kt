package com.elon.app

import android.content.Context
import android.media.MediaRecorder
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

internal class VoiceAudioRecorder(private val context: Context) {
    private var recorder: MediaRecorder? = null
    private var outputFile: File? = null

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
        }.onFailure {
            runCatching { created.release() }
            file.delete()
        }.isSuccess
    }

    fun stopToAttachment(): PendingAttachment? {
        val activeRecorder = recorder ?: return null
        val file = outputFile
        recorder = null
        outputFile = null
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
            file = file
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

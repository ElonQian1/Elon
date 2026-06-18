package com.elon.chatvoice

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.MediaRecorder
import android.os.SystemClock
import java.io.File

class ChatVoiceRecorder(
    private val context: Context,
    private val options: ChatVoiceHoldOptions = ChatVoiceInteractionContract.holdOptions,
    private val eventSink: ChatVoiceEventSink? = null,
) {
    private val appContext = context.applicationContext
    private var recorder: MediaRecorder? = null
    private var outputFile: File? = null
    private var startedAt: Long = 0L

    val isRecording: Boolean
        get() = recorder != null

    fun start(): Result<Unit> {
        cancel()
        if (appContext.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            return Result.failure(IllegalStateException("record audio permission denied"))
        }
        val file = File(voiceCacheDir(), "chat_voice_${System.currentTimeMillis()}.m4a")
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
        }
    }

    fun recordedDurationSeconds(): Int {
        if (recorder == null) return 0
        return ((SystemClock.elapsedRealtime() - startedAt) / 1000L).toInt().coerceAtLeast(1)
    }

    fun stop(): Result<RecordedVoice> {
        val durationMillis = recordedDurationMillis()
        val duration = recordedDurationSeconds()
        val activeRecorder = recorder
        val file = outputFile
        recorder = null
        outputFile = null
        startedAt = 0L
        if (activeRecorder == null || file == null) {
            return Result.failure(IllegalStateException("recorder is not active"))
        }
        val stopped = runCatching { activeRecorder.stop() }
        runCatching { activeRecorder.release() }
        if (stopped.isFailure ||
            durationMillis < options.minRecordDurationMs ||
            !file.isFile ||
            file.length() <= options.minVoiceBytes
        ) {
            file.delete()
            eventSink?.onVoiceEvent(ChatVoiceEvent.TooShort(options.minRecordDurationMs, options.minVoiceBytes))
            return Result.failure(IllegalStateException("voice recording is too short"))
        }
        return Result.success(RecordedVoice(file = file, durationSeconds = duration, durationMillis = durationMillis))
    }

    fun cancel() {
        val activeRecorder = recorder
        val file = outputFile
        recorder = null
        outputFile = null
        startedAt = 0L
        if (activeRecorder != null) {
            runCatching { activeRecorder.stop() }
            runCatching { activeRecorder.release() }
        }
        file?.delete()
    }

    private fun voiceCacheDir(): File =
        File(appContext.cacheDir, "elon_chat_voice").apply { mkdirs() }

    private fun recordedDurationMillis(): Long =
        if (startedAt <= 0L) 0L else SystemClock.elapsedRealtime() - startedAt
}

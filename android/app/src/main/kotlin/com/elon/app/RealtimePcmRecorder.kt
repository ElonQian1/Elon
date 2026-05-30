package com.elon.app

import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/**
 * 实时 PCM16 麦克风采集器（24kHz / mono），用于"实时语音 → 服务器 WebSocket"管线。
 *
 * 设计上和 [VoiceAudioRecorder]（录 m4a 走附件上传）完全独立：
 *  - 本类只面向流式管线，不写本地文件
 *  - 输出原始 PCM16 LE bytes，符合服务端 voice_config.rs 约束
 *  - 每 [FRAME_MS] 毫秒回调一次 onChunk，调用方负责通过 WebSocket 推送
 */
internal class RealtimePcmRecorder(
    private val onChunk: (ByteArray) -> Unit,
    private val onError: (String) -> Unit,
) {
    companion object {
        const val SAMPLE_RATE_HZ = 24_000
        const val FRAME_MS = 40
        private const val TAG = "RealtimePcmRecorder"
    }

    private val channelConfig = AudioFormat.CHANNEL_IN_MONO
    private val audioFormat = AudioFormat.ENCODING_PCM_16BIT
    private val bytesPerSample = 2

    private var audioRecord: AudioRecord? = null
    private var captureJob: Job? = null

    @Volatile
    var isRecording: Boolean = false
        private set

    /**
     * 启动采集。需要调用方提前申请 RECORD_AUDIO 权限。
     */
    @SuppressWarnings("MissingPermission")
    fun start(scope: CoroutineScope): Boolean {
        if (isRecording) return true
        val minBuf = AudioRecord.getMinBufferSize(SAMPLE_RATE_HZ, channelConfig, audioFormat)
        if (minBuf <= 0) {
            onError("AudioRecord.getMinBufferSize 失败：$minBuf")
            return false
        }
        val frameSamples = SAMPLE_RATE_HZ * FRAME_MS / 1000
        val frameBytes = frameSamples * bytesPerSample
        val bufBytes = maxOf(minBuf, frameBytes * 4)

        val recorder = try {
            AudioRecord(
                MediaRecorder.AudioSource.VOICE_RECOGNITION,
                SAMPLE_RATE_HZ,
                channelConfig,
                audioFormat,
                bufBytes,
            )
        } catch (t: Throwable) {
            onError("创建 AudioRecord 失败：${t.message}")
            return false
        }
        if (recorder.state != AudioRecord.STATE_INITIALIZED) {
            recorder.release()
            onError("AudioRecord 未初始化（state=${recorder.state}）")
            return false
        }

        audioRecord = recorder
        isRecording = true
        recorder.startRecording()

        captureJob = scope.launch(Dispatchers.IO) {
            val frame = ByteArray(frameBytes)
            try {
                while (isRecording) {
                    val read = recorder.read(frame, 0, frame.size)
                    if (read <= 0) {
                        // 防止 CPU 紧循环：硬件瞬时错误或音频资源竞争时 read() 立即返回
                        // 负数：AudioRecord.ERROR / ERROR_INVALID_OPERATION / ERROR_BAD_VALUE
                        // 0：极少情况下的空返回
                        if (read < 0) Log.w(TAG, "recorder.read 错误码: $read（可能是音频资源被抢占）")
                        kotlinx.coroutines.delay(5L)
                        continue
                    }
                    if (read == frame.size) {
                        onChunk(frame.copyOf())
                    } else {
                        onChunk(frame.copyOf(read - read % 2))
                    }
                }
            } catch (t: Throwable) {
                Log.w(TAG, "采集循环异常", t)
                onError("采集异常：${t.message}")
            }
        }
        return true
    }

    fun stop() {
        if (!isRecording) return
        isRecording = false
        captureJob?.cancel()
        captureJob = null
        runCatching { audioRecord?.stop() }
        runCatching { audioRecord?.release() }
        audioRecord = null
    }
}

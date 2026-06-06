// infrastructure/voice/VoiceRecognitionHelper.kt
package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.util.Log
import com.elon.app.AgentVoiceBridge

/**
 * 🎤 语音识别辅助类（统一管线）
 *
 * 底层委托给 [AgentVoiceBridge]，与主聊天区 / 悬浮球走同一条
 * RecognitionEngineSelector → StreamingASR 管线，自动优先使用
 * 小米 mibrain（com.xiaomi.aiasst.vision），失败时自动回退到
 * Google / 云端 ASR。
 *
 * 公开 API 保持不变，调用方无需任何修改。
 */
class VoiceRecognitionHelper(context: Context) {

    companion object {
        private const val TAG = "VoiceRecognition"
    }

    private val bridge = AgentVoiceBridge(context)

    var isListening: Boolean = false
        private set

    // 回调接口（与原接口保持一致）
    var onResult: ((String) -> Unit)? = null
    var onPartialResult: ((String) -> Unit)? = null
    var onError: ((String) -> Unit)? = null
    var onListeningStateChanged: ((Boolean) -> Unit)? = null

    init {
        bridge.onFinal = { text ->
            Log.i(TAG, "识别完成: $text")
            isListening = false
            onListeningStateChanged?.invoke(false)
            onResult?.invoke(text)
        }
        bridge.onPartial = { text ->
            onPartialResult?.invoke(text)
        }
        bridge.onError = { msg ->
            Log.e(TAG, "识别错误: $msg")
            isListening = false
            onListeningStateChanged?.invoke(false)
            onError?.invoke(msg)
        }
        bridge.onStart = {
            isListening = true
            onListeningStateChanged?.invoke(true)
        }
        bridge.onEnd = {
            if (isListening) {
                isListening = false
                onListeningStateChanged?.invoke(false)
            }
        }
    }
    
    fun isAvailable(): Boolean = true // AgentVoiceBridge 内部处理所有回退

    /** 预热：提前完成引擎服务绑定，减少首次识别延迟。 */
    fun initialize() {
        bridge.prewarm()
    }

    fun startListening() {
        if (isListening) {
            stopListening()
            return
        }
        Log.i(TAG, "开始语音识别")
        bridge.start()
    }

    fun stopListening() {
        bridge.stop()
        // isListening 由 onEnd 回调更新，这里兜底一次
        isListening = false
        onListeningStateChanged?.invoke(false)
        Log.i(TAG, "停止语音识别")
    }

    fun destroy() {
        bridge.destroy()
        isListening = false
        Log.i(TAG, "语音识别器已释放")
    }
}


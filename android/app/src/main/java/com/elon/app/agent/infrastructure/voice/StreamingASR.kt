// infrastructure/voice/StreamingASR.kt
// module: infrastructure/voice | layer: infrastructure | role: streaming-asr
// summary: 流式语音识别 - 支持边说边识别，结合智能VAD

package com.elon.app.agent.infrastructure.voice

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.util.Log

/**
 * 🎤 流式语音识别器
 * 
 * 特性：
 * 1. 实时返回部分识别结果
 * 2. 集成智能VAD，边说边判断是否完整
 * 3. 支持打断检测
 * 4. 音量回调（用于动画）
 */
class StreamingASR(private val context: Context) {
    
    companion object {
        private const val TAG = "StreamingASR"
        
        // 静音检测间隔
        private const val SILENCE_CHECK_INTERVAL_MS = 100L
    }
    
    // ==================== 组件 ====================
    
    private var speechRecognizer: SpeechRecognizer? = null
    private val smartVAD = SmartVAD()
    private val handler = Handler(Looper.getMainLooper())
    
    // ==================== 状态 ====================
    
    var isListening: Boolean = false
        private set
    
    private var lastResultTime: Long = 0
    private var currentPartialResult: String = ""
    private var silenceCheckRunnable: Runnable? = null
    /** onFinalResult 已经触发过一次，防止安全网和 onResults 双重回调 */
    private var finalDelivered: Boolean = false
    /** 诊断：本次会话麦克风峰值音量（onRmsChanged 最大值），用于判断麦克风是否真的收到声音 */
    private var maxRmsThisSession: Float = -100f
    /** 诊断：本次会话小米/系统引擎是否检测到语音开始（onBeginningOfSpeech） */
    private var sawBeginningOfSpeech: Boolean = false
    
    // ==================== 回调 ====================
    
    var callback: StreamingASRCallback? = null
    
    // ==================== 配置 ====================
    
    /** 是否启用智能VAD（否则使用系统默认） */
    var useSmartVAD: Boolean = true
    
    /** 语言 */
    var language: String = "zh-CN"

    /**
     * 指定使用的识别服务组件。
     *  - null：调用 [SpeechRecognizer.createSpeechRecognizer]（系统默认）；
     *  - 非 null：调用 [SpeechRecognizer.createSpeechRecognizer]（context, component）强制指定。
     * 修改后必须调用 [destroy] 或 [resetEngine] 才会在下次 [startListening] 生效。
     */
    var engineComponent: ComponentName? = null

    /** SpeechRecognizer 是否已创建（服务绑定是否已启动）。 */
    val isInitialized: Boolean get() = speechRecognizer != null

    // ==================== 公开方法 ====================

    /**
     * 初始化
     */
    fun initialize() {
        if (speechRecognizer != null) return

        val component = engineComponent
        speechRecognizer = if (component != null) {
            Log.i(TAG, "使用指定引擎: ${component.flattenToShortString()}")
            SpeechRecognizer.createSpeechRecognizer(context, component)
        } else {
            SpeechRecognizer.createSpeechRecognizer(context)
        }
        speechRecognizer?.setRecognitionListener(createListener())
        Log.i(TAG, "✅ 流式ASR初始化完成 (engine=${component?.flattenToShortString() ?: "<system-default>"})")
    }

    /**
     * 释放当前 recognizer 实例（保留回调与配置），下次 [startListening] 会按 [engineComponent] 重新创建。
     * 用于在多个引擎之间切换重试。
     */
    fun resetEngine() {
        stopSilenceCheck()
        speechRecognizer?.destroy()
        speechRecognizer = null
        isListening = false
        Log.i(TAG, "🔄 引擎已重置，等待重新初始化")
    }
    
    /**
     * 开始识别
     */
    fun startListening() {
        if (isListening) {
            Log.w(TAG, "已在识别中")
            return
        }
        
        initialize()
        
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, language)
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)  // 关键：启用部分结果
            putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)


            // 如果使用智能VAD，延长系统默认静音时间
            if (useSmartVAD) {
                putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_COMPLETE_SILENCE_LENGTH_MILLIS, 5000L)
                putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_POSSIBLY_COMPLETE_SILENCE_LENGTH_MILLIS, 3000L)
                putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_MINIMUM_LENGTH_MILLIS, 1000L)
            }
        }
        
        try {
            speechRecognizer?.startListening(intent)
            isListening = true
            lastResultTime = System.currentTimeMillis()
            currentPartialResult = ""
            finalDelivered = false
            maxRmsThisSession = -100f
            sawBeginningOfSpeech = false

            // 启动静音检测
            if (useSmartVAD) {
                startSilenceCheck()
            }

            Log.i(TAG, "🎤 开始流式识别 (smartVAD=$useSmartVAD)")
        } catch (e: Exception) {
            Log.e(TAG, "启动失败", e)
            callback?.onError("启动失败: ${e.message}")
        }
    }
    
    /**
     * 停止识别
     */
    fun stopListening() {
        stopSilenceCheck()
        speechRecognizer?.stopListening()
        isListening = false
        Log.i(TAG, "🛑 停止识别")
    }
    
    /**
     * 取消识别
     */
    fun cancel() {
        stopSilenceCheck()
        speechRecognizer?.cancel()
        isListening = false
        currentPartialResult = ""
        Log.i(TAG, "❌ 取消识别")
    }
    
    /**
     * 释放资源
     */
    fun destroy() {
        stopSilenceCheck()
        speechRecognizer?.destroy()
        speechRecognizer = null
        isListening = false
        Log.i(TAG, "🧹 ASR资源已释放")
    }
    
    // ==================== 内部方法 ====================
    
    /**
     * 创建识别监听器
     */
    private fun createListener() = object : RecognitionListener {
        
        override fun onReadyForSpeech(params: Bundle?) {
            Log.d(TAG, "📢 准备就绪")
            callback?.onReady()
        }
        
        override fun onBeginningOfSpeech() {
            Log.i(TAG, "🎤 检测到语音开始")
            sawBeginningOfSpeech = true
            lastResultTime = System.currentTimeMillis()
            callback?.onSpeechStart()
        }
        
        override fun onRmsChanged(rmsdB: Float) {
            // 音量变化，范围大约 -2 到 10
            if (rmsdB > maxRmsThisSession) maxRmsThisSession = rmsdB
            val normalizedVolume = ((rmsdB + 2) / 12).coerceIn(0f, 1f)
            callback?.onVolumeChanged(normalizedVolume)
        }
        
        override fun onBufferReceived(buffer: ByteArray?) {}
        
        override fun onEndOfSpeech() {
            Log.d(TAG, "🔇 语音结束 (系统VAD)")
            if (!useSmartVAD) {
                callback?.onSpeechEnd()
            }
            // 如果使用智能VAD，让我们的检测来决定
        }
        
        override fun onError(error: Int) {
            isListening = false
            stopSilenceCheck()
            
            val errorMsg = when (error) {
                SpeechRecognizer.ERROR_AUDIO -> "录音错误"
                SpeechRecognizer.ERROR_CLIENT -> "客户端错误"
                SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "权限不足"
                SpeechRecognizer.ERROR_NETWORK -> "网络错误"
                SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "网络超时"
                SpeechRecognizer.ERROR_NO_MATCH -> "未识别到语音"
                SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "识别器忙"
                SpeechRecognizer.ERROR_SERVER -> "服务器错误"
                SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "语音超时"
                11 -> "服务连接被断开"           // ERROR_SERVER_DISCONNECTED (API 31+)
                12 -> "语言不支持"               // ERROR_LANGUAGE_NOT_SUPPORTED
                13 -> "语言未下载"               // ERROR_LANGUAGE_UNAVAILABLE
                14 -> "无法检查语言支持"          // ERROR_CANNOT_CHECK_SUPPORT
                else -> "未知错误: $error"
            }
            
            Log.e(TAG, "❌ 错误: $errorMsg (code=$error) | 诊断: 峰值音量=${"%.1f".format(maxRmsThisSession)}dB, 检测到语音开始=$sawBeginningOfSpeech, 有部分结果=${currentPartialResult.isNotEmpty()}")

            // NO_MATCH 不一定是错误，可能是用户还没说
            if (error != SpeechRecognizer.ERROR_NO_MATCH) {
                callback?.onErrorCode(error, errorMsg)
                callback?.onError(errorMsg)
            } else if (currentPartialResult.isNotEmpty()) {
                // 有部分结果时，当作最终结果
                callback?.onFinalResult(currentPartialResult)
            } else {
                // NO_MATCH 且无任何部分结果：也上报码，让上层决定是否切换引擎
                callback?.onErrorCode(error, errorMsg)
            }
        }
        
        override fun onResults(results: Bundle?) {
            isListening = false
            stopSilenceCheck()

            val matches = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
            val finalResult = matches?.firstOrNull() ?: currentPartialResult

            if (finalResult.isNotEmpty() && !finalDelivered) {
                finalDelivered = true
                Log.i(TAG, "✅ 最终结果: $finalResult")
                callback?.onFinalResult(finalResult)
            }
        }
        
        override fun onPartialResults(partialResults: Bundle?) {
            val matches = partialResults?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
            val partialText = matches?.firstOrNull() ?: return
            
            if (partialText.isEmpty()) return
            
            currentPartialResult = partialText
            lastResultTime = System.currentTimeMillis()
            
            // 计算置信度（简单估算）
            val confidence = if (partialText.length > 3) 0.8f else 0.5f
            
            Log.d(TAG, "📝 部分结果: $partialText")
            callback?.onPartialResult(partialText, confidence)
        }
        
        override fun onEvent(eventType: Int, params: Bundle?) {}
    }
    
    /**
     * 启动静音检测
     */
    private fun startSilenceCheck() {
        silenceCheckRunnable = object : Runnable {
            override fun run() {
                if (!isListening) return
                
                val silenceDuration = System.currentTimeMillis() - lastResultTime
                
                // 使用智能VAD判断
                val decision = smartVAD.shouldEndInput(currentPartialResult, silenceDuration)
                
                if (decision.shouldEnd && currentPartialResult.isNotEmpty()) {
                    Log.i(TAG, "🎯 智能VAD决定结束: ${decision.completeness.reason}")

                    // 通知上层语音结束（UI 状态变化），但不立即停止识别器
                    callback?.onSpeechEnd()

                    // 让引擎把缓冲区内的语音全部识别完再返回 onResults。
                    // 不能在这里直接 onFinalResult(partial)——MiBrain 等引擎的 final
                    // 比 partial 晚 600ms+ 才到，200ms 强制提交会截断句子。
                    stopListening()

                    // 安全网：2000ms 后若 onResults 仍未到来（引擎超时/失败），
                    // 才降级用 partial 兜底。
                    val savedPartial = currentPartialResult
                    handler.postDelayed({
                        if (!finalDelivered && savedPartial.isNotEmpty()) {
                            finalDelivered = true
                            Log.w(TAG, "⚠️ onResults 超时，降级使用 partial: $savedPartial")
                            callback?.onFinalResult(savedPartial)
                        }
                    }, 2000)
                } else {
                    // 继续检测
                    handler.postDelayed(this, SILENCE_CHECK_INTERVAL_MS)
                }
            }
        }
        
        handler.postDelayed(silenceCheckRunnable!!, SILENCE_CHECK_INTERVAL_MS)
    }
    
    /**
     * 停止静音检测
     */
    private fun stopSilenceCheck() {
        silenceCheckRunnable?.let { handler.removeCallbacks(it) }
        silenceCheckRunnable = null
    }
}

/**
 * 流式ASR回调接口
 */
interface StreamingASRCallback {
    /** 准备就绪 */
    fun onReady() {}

    /** 检测到语音开始 */
    fun onSpeechStart()

    /** 收到部分结果 */
    fun onPartialResult(text: String, confidence: Float)

    /** 收到最终结果 */
    fun onFinalResult(text: String)

    /** 检测到语音结束 */
    fun onSpeechEnd()

    /** 音量变化 (0-1) */
    fun onVolumeChanged(volume: Float) {}

    /** 错误（文字描述） */
    fun onError(message: String)

    /**
     * 错误（携带 [android.speech.SpeechRecognizer] 的 ERROR_* 错误码）。
     * 默认空实现，上层若需要按码切换引擎可覆写本方法。
     */
    fun onErrorCode(code: Int, message: String) {}
}

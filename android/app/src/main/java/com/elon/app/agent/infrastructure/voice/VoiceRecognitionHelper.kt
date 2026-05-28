// infrastructure/voice/VoiceRecognitionHelper.kt
package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.util.Log

/**
 * 🎤 语音识别辅助类
 * 
 * 支持实时语音转文字，用于语音输入任务目标
 */
class VoiceRecognitionHelper(private val context: Context) {
    
    companion object {
        private const val TAG = "VoiceRecognition"
    }
    
    private var speechRecognizer: SpeechRecognizer? = null
    var isListening: Boolean = false
        private set
    
    // 回调接口
    var onResult: ((String) -> Unit)? = null
    var onPartialResult: ((String) -> Unit)? = null
    var onError: ((String) -> Unit)? = null
    var onListeningStateChanged: ((Boolean) -> Unit)? = null
    
    /**
     * 检查设备是否支持语音识别
     * 注意：小米等手机可能返回 false 但实际支持，所以这个检查仅供参考
     */
    fun isAvailable(): Boolean {
        val available = SpeechRecognizer.isRecognitionAvailable(context)
        Log.i(TAG, "语音识别可用性检查: $available")
        return available
    }
    
    /**
     * 初始化语音识别器
     */
    fun initialize() {
        if (speechRecognizer != null) return
        
        speechRecognizer = SpeechRecognizer.createSpeechRecognizer(context)
        speechRecognizer?.setRecognitionListener(createListener())
        Log.i(TAG, "语音识别器已初始化")
    }
    
    /**
     * 开始语音识别
     * 注意：即使 isAvailable() 返回 false，也尝试启动（小米等手机兼容）
     */
    fun startListening() {
        // 不再检查 isAvailable()，直接尝试启动
        // 因为小米等手机可能返回 false 但实际可用
        
        if (isListening) {
            stopListening()
            return
        }
        
        initialize()
        
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, "zh-CN")
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
            putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
            // 语音输入提示
            putExtra(RecognizerIntent.EXTRA_PROMPT, "请说出你的任务目标...")
            // 🆕 延长静音等待时间（毫秒）- 让用户说完整句话
            putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_COMPLETE_SILENCE_LENGTH_MILLIS, 3000L)
            putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_POSSIBLY_COMPLETE_SILENCE_LENGTH_MILLIS, 2500L)
            putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_MINIMUM_LENGTH_MILLIS, 2000L)
        }
        
        try {
            speechRecognizer?.startListening(intent)
            isListening = true
            onListeningStateChanged?.invoke(true)
            Log.i(TAG, "开始语音识别")
        } catch (e: Exception) {
            Log.e(TAG, "启动语音识别失败", e)
            onError?.invoke("启动失败: ${e.message}")
        }
    }
    
    /**
     * 停止语音识别
     */
    fun stopListening() {
        speechRecognizer?.stopListening()
        isListening = false
        onListeningStateChanged?.invoke(false)
        Log.i(TAG, "停止语音识别")
    }
    
    /**
     * 释放资源
     */
    fun destroy() {
        speechRecognizer?.destroy()
        speechRecognizer = null
        isListening = false
        Log.i(TAG, "语音识别器已释放")
    }
    
    /**
     * 创建识别监听器
     */
    private fun createListener() = object : RecognitionListener {
        
        override fun onReadyForSpeech(params: Bundle?) {
            Log.d(TAG, "准备就绪，请说话...")
        }
        
        override fun onBeginningOfSpeech() {
            Log.d(TAG, "检测到语音开始")
        }
        
        override fun onRmsChanged(rmsdB: Float) {
            // 音量变化，可用于显示波形动画
        }
        
        override fun onBufferReceived(buffer: ByteArray?) {}
        
        override fun onEndOfSpeech() {
            Log.d(TAG, "语音结束")
            isListening = false
            onListeningStateChanged?.invoke(false)
        }
        
        override fun onError(error: Int) {
            isListening = false
            onListeningStateChanged?.invoke(false)
            
            val errorMessage = when (error) {
                SpeechRecognizer.ERROR_AUDIO -> "录音错误"
                SpeechRecognizer.ERROR_CLIENT -> "客户端错误"
                SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "权限不足，请授予麦克风权限"
                SpeechRecognizer.ERROR_NETWORK -> "网络错误"
                SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "网络超时"
                SpeechRecognizer.ERROR_NO_MATCH -> "未识别到语音"
                SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "识别器忙"
                SpeechRecognizer.ERROR_SERVER -> "服务器错误"
                SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "语音超时"
                else -> "未知错误 ($error)"
            }
            
            Log.e(TAG, "识别错误: $errorMessage")
            onError?.invoke(errorMessage)
        }
        
        override fun onResults(results: Bundle?) {
            val matches = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
            val text = matches?.firstOrNull() ?: ""
            
            if (text.isNotEmpty()) {
                Log.i(TAG, "识别结果: $text")
                onResult?.invoke(text)
            }
            
            isListening = false
            onListeningStateChanged?.invoke(false)
        }
        
        override fun onPartialResults(partialResults: Bundle?) {
            val matches = partialResults?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
            val text = matches?.firstOrNull() ?: ""
            
            if (text.isNotEmpty()) {
                Log.d(TAG, "部分结果: $text")
                onPartialResult?.invoke(text)
            }
        }
        
        override fun onEvent(eventType: Int, params: Bundle?) {}
    }
}

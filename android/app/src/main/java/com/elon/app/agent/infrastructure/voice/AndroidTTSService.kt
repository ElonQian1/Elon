// infrastructure/voice/AndroidTTSService.kt
// module: infrastructure/voice | layer: infrastructure | role: tts-implementation
// summary: Android 原生 TTS 实现 - 使用系统语音合成引擎

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.util.Log
import java.util.*

/**
 * 🔊 Android 原生 TTS 服务
 * 
 * 使用 Android 内置的 TextToSpeech 引擎
 * 
 * 优点：
 * - 无需网络
 * - 延迟低
 * - 兼容性好
 * 
 * 缺点：
 * - 音质一般
 * - 情感表达有限
 */
class AndroidTTSService(context: Context) : TextToSpeechService {
    
    companion object {
        private const val TAG = "AndroidTTS"
    }
    
    private var tts: TextToSpeech? = null
    private var isInitialized = false
    private var pendingCallback: (() -> Unit)? = null
    
    override val isSpeaking: Boolean
        get() = tts?.isSpeaking == true
    
    init {
        tts = TextToSpeech(context) { status ->
            if (status == TextToSpeech.SUCCESS) {
                val result = tts?.setLanguage(Locale.CHINESE)
                if (result == TextToSpeech.LANG_MISSING_DATA || 
                    result == TextToSpeech.LANG_NOT_SUPPORTED) {
                    Log.w(TAG, "中文语音包不可用，使用默认语言")
                }
                
                // 设置语速
                tts?.setSpeechRate(1.0f)
                
                // 设置音调
                tts?.setPitch(1.0f)
                
                // 设置播放监听
                tts?.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                    override fun onStart(utteranceId: String?) {
                        Log.d(TAG, "▶️ TTS 开始播放: $utteranceId")
                    }
                    
                    override fun onDone(utteranceId: String?) {
                        Log.d(TAG, "✅ TTS 播放完成: $utteranceId")
                        pendingCallback?.invoke()
                        pendingCallback = null
                    }
                    
                    @Deprecated("Deprecated in API 21")
                    override fun onError(utteranceId: String?) {
                        Log.e(TAG, "❌ TTS 播放错误: $utteranceId")
                        pendingCallback?.invoke()
                        pendingCallback = null
                    }
                    
                    override fun onError(utteranceId: String?, errorCode: Int) {
                        Log.e(TAG, "❌ TTS 播放错误: $utteranceId, code: $errorCode")
                        pendingCallback?.invoke()
                        pendingCallback = null
                    }
                })
                
                isInitialized = true
                Log.i(TAG, "✅ TTS 初始化成功")
            } else {
                Log.e(TAG, "❌ TTS 初始化失败: $status")
            }
        }
    }
    
    override fun speak(text: String, onComplete: (() -> Unit)?) {
        if (!isInitialized) {
            Log.w(TAG, "TTS 未初始化，跳过播放")
            onComplete?.invoke()
            return
        }
        
        if (text.isBlank()) {
            onComplete?.invoke()
            return
        }
        
        pendingCallback = onComplete
        
        val params = android.os.Bundle().apply {
            putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, 1.0f)
        }
        
        val utteranceId = "utterance_${System.currentTimeMillis()}"
        
        tts?.speak(text, TextToSpeech.QUEUE_FLUSH, params, utteranceId)
        Log.d(TAG, "🔊 播放: $text")
    }
    
    override fun stop() {
        tts?.stop()
        pendingCallback = null
        Log.d(TAG, "⏹️ 停止播放")
    }
    
    override fun destroy() {
        tts?.shutdown()
        tts = null
        isInitialized = false
        Log.i(TAG, "🧹 TTS 资源释放")
    }
}

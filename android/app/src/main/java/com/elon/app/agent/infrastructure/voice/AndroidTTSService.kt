// infrastructure/voice/AndroidTTSService.kt
// module: infrastructure/voice | layer: infrastructure | role: tts-implementation
// summary: 悬浮球 TTS 适配器，优先复用主应用情绪 TTS

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.util.Log
import com.elon.app.VoiceSpeaker

/**
 * 悬浮球语音输出服务。
 *
 * 这里故意保留 AndroidTTSService 名称，避免改动对话适配器装配代码；
 * 内部委托给主应用的 VoiceSpeaker，从而复用服务器情绪 TTS、女声情绪 profile
 * 和系统 TTS 降级逻辑。
 */
class AndroidTTSService(context: Context) : TextToSpeechService {

    companion object {
        private const val TAG = "FloatingEmotionTTS"
    }

    private val speaker = VoiceSpeaker(context, respectUserToggle = false)

    override val isSpeaking: Boolean
        get() = speaker.isSpeaking

    override fun speak(text: String, onComplete: (() -> Unit)?) {
        val content = text.trim()
        if (content.isEmpty()) {
            onComplete?.invoke()
            return
        }
        Log.d(TAG, "播放悬浮球情绪 TTS: ${content.take(60)}")
        speaker.speak(content, onDone = onComplete)
    }

    override fun stop() {
        speaker.stop()
        Log.d(TAG, "停止悬浮球情绪 TTS")
    }

    override fun destroy() {
        speaker.release()
        Log.i(TAG, "悬浮球情绪 TTS 资源释放")
    }
}

package com.elon.app

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.util.Log
import java.util.Locale
import java.util.UUID

/**
 * Android 系统 TTS 封装（MVP 版）。
 *
 * 用途：AI 完成回复后，通过系统语音引擎朗读回复文本。
 * 语言：简体中文（zh-CN），不可用时回退系统默认语言。
 *
 * 生命周期：与 MainSpeechInputActions 保持一致，在 destroy() 时调用 [release]。
 */
internal class VoiceSpeaker(context: Context) : TextToSpeech.OnInitListener {

    companion object {
        private const val TAG = "VoiceSpeaker"
        private const val MAX_SPEAK_CHARS = 200
        private const val PREFS_NAME = "elon"
        private const val KEY_TTS_ENABLED = "tts_speak_enabled"

        fun isTtsEnabled(context: Context): Boolean =
            context.applicationContext
                .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .getBoolean(KEY_TTS_ENABLED, false)

        fun setTtsEnabled(context: Context, enabled: Boolean) {
            context.applicationContext
                .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit().putBoolean(KEY_TTS_ENABLED, enabled).apply()
        }
    }

    private val appContext: Context = context.applicationContext
    private var tts: TextToSpeech? = TextToSpeech(appContext, this)
    private var ready = false

    /** 上一条 utterance 是否还在播放（用于打断判断）。 */
    val isSpeaking: Boolean get() = tts?.isSpeaking == true

    override fun onInit(status: Int) {
        if (status != TextToSpeech.SUCCESS) {
            Log.w(TAG, "TTS 初始化失败 status=$status")
            return
        }
        val engine = tts ?: return
        val result = engine.setLanguage(Locale.SIMPLIFIED_CHINESE)
        if (result == TextToSpeech.LANG_MISSING_DATA || result == TextToSpeech.LANG_NOT_SUPPORTED) {
            Log.w(TAG, "zh-CN 不可用，使用系统默认语言")
            engine.setLanguage(Locale.getDefault())
        }
        engine.setSpeechRate(1.05f)  // 稍微快一点更自然
        engine.setPitch(1.0f)
        engine.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {}
            override fun onDone(utteranceId: String?) {}
            @Deprecated("Deprecated in API 21", ReplaceWith("onError(utteranceId, errorCode)"))
            override fun onError(utteranceId: String?) {}
        })
        ready = true
        Log.d(TAG, "TTS 初始化成功")
    }

    /**
     * 朗读文本。若上一条还在朗读，立即打断并播放新内容（QUEUE_FLUSH）。
     * 超长文本截断到 [MAX_SPEAK_CHARS] 字符，避免朗读时间过长。
     */
    fun speak(text: String) {
        if (!ready) return
        if (!isTtsEnabled(appContext)) return
        val engine = tts ?: return
        val content = text.trim().take(MAX_SPEAK_CHARS)
        if (content.isEmpty()) return
        val params = Bundle()
        engine.speak(content, TextToSpeech.QUEUE_FLUSH, params, UUID.randomUUID().toString())
    }

    /** 立即停止当前朗读（用户开始新一轮语音输入时调用）。 */
    fun stop() {
        if (tts?.isSpeaking == true) tts?.stop()
    }

    /** 释放资源（Activity onDestroy）。 */
    fun release() {
        tts?.stop()
        tts?.shutdown()
        tts = null
        ready = false
    }
}

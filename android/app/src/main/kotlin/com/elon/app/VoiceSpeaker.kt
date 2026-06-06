package com.elon.app

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.speech.tts.Voice
import android.util.Log
import java.util.Locale
import java.util.UUID

/**
 * Android 系统 TTS 封装（MVP 版）。
 *
 * 用途：AI 完成回复后，通过系统语音引擎朗读回复文本。
 * 语言：简体中文（zh-CN），不可用时回退系统默认语言。
 * 情感：按文本内容选择轻量情感档位，映射到系统 TTS 的语速和音高。
 *
 * 生命周期：与 MainSpeechInputActions 保持一致，在 destroy() 时调用 [release]。
 */
internal class VoiceSpeaker(
    context: Context,
    private val respectUserToggle: Boolean = true
) : TextToSpeech.OnInitListener {

    companion object {
        private const val TAG = "VoiceSpeaker"
        private const val MAX_SPEAK_CHARS = 200

        fun isTtsEnabled(context: Context): Boolean =
            VoiceTtsPreferences.isSpeakEnabled(context)

        fun setTtsEnabled(context: Context, enabled: Boolean) {
            VoiceTtsPreferences.setSpeakEnabled(context, enabled)
        }
    }

    private val appContext: Context = context.applicationContext
    private var tts: TextToSpeech? = TextToSpeech(appContext, this)
    private var ready = false
    private var pendingText: String? = null
    private var pendingProfile: VoiceTtsProfile? = null
    private var pendingDone: (() -> Unit)? = null
    private var activeDone: (() -> Unit)? = null
    private var preferredVoiceApplied = false
    private val serverTtsPlayer = VoiceServerTtsPlayer(appContext)

    /** 上一条 utterance 是否还在播放（用于打断判断）。 */
    val isSpeaking: Boolean get() = tts?.isSpeaking == true || serverTtsPlayer.isSpeaking

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
        applyPreferredVoice(engine)
        applyProfile(engine, VoiceTtsEmotion.profileFor(""))
        engine.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {}
            override fun onDone(utteranceId: String?) {
                finishSpeakCallback()
            }
            @Deprecated("Deprecated in API 21", ReplaceWith("onError(utteranceId, errorCode)"))
            override fun onError(utteranceId: String?) {
                finishSpeakCallback()
            }
        })
        ready = true
        pendingText?.let { text ->
            val profile = pendingProfile
            val done = pendingDone
            pendingText = null
            pendingProfile = null
            pendingDone = null
            speak(text, profile, done)
        }
        Log.d(TAG, "TTS 初始化成功")
    }

    /**
     * 朗读文本。若上一条还在朗读，立即打断并播放新内容（QUEUE_FLUSH）。
     * 超长文本截断到 [MAX_SPEAK_CHARS] 字符，避免朗读时间过长。
     */
    fun speak(
        text: String,
        profile: VoiceTtsProfile? = null,
        onDone: (() -> Unit)? = null
    ) {
        if (respectUserToggle && !isTtsEnabled(appContext)) {
            onDone?.invoke()
            return
        }
        val content = text.trim().take(MAX_SPEAK_CHARS)
        if (content.isEmpty()) {
            onDone?.invoke()
            return
        }
        val engine = tts
        if (!ready || engine == null) {
            pendingText = content
            pendingProfile = profile
            pendingDone = onDone
            return
        }
        pendingText = null
        pendingProfile = null
        pendingDone = null
        serverTtsPlayer.stop()
        if (tts?.isSpeaking == true) tts?.stop()
        val resolvedProfile = profile ?: VoiceTtsEmotion.profileFor(content)
        activeDone = onDone
        Log.d(TAG, "尝试服务器情绪 TTS profile=${resolvedProfile.id}")
        if (serverTtsPlayer.trySpeak(
                text = content,
                profile = resolvedProfile,
                onDone = { finishSpeakCallback() },
                onFallback = {
                    Log.w(TAG, "服务器情绪 TTS 不可用，降级系统 TTS profile=${resolvedProfile.id}")
                    speakWithSystem(engine, content, resolvedProfile, onDone)
                }
            )
        ) {
            return
        }
        Log.w(TAG, "服务器情绪 TTS 被跳过，降级系统 TTS profile=${resolvedProfile.id}")
        speakWithSystem(engine, content, resolvedProfile, onDone)
    }

    private fun speakWithSystem(
        engine: TextToSpeech,
        content: String,
        profile: VoiceTtsProfile,
        onDone: (() -> Unit)?
    ) {
        activeDone = onDone
        applyProfile(engine, profile)
        val params = Bundle()
        val result = engine.speak(content, TextToSpeech.QUEUE_FLUSH, params, UUID.randomUUID().toString())
        if (result == TextToSpeech.ERROR) finishSpeakCallback()
    }

    /** 立即停止当前朗读（用户开始新一轮语音输入时调用）。 */
    fun stop() {
        pendingText = null
        pendingProfile = null
        pendingDone = null
        activeDone = null
        serverTtsPlayer.stop()
        if (tts?.isSpeaking == true) tts?.stop()
    }

    /** 释放资源（Activity onDestroy）。 */
    fun release() {
        pendingText = null
        pendingProfile = null
        pendingDone = null
        activeDone = null
        serverTtsPlayer.release()
        tts?.stop()
        tts?.shutdown()
        tts = null
        ready = false
        preferredVoiceApplied = false
    }

    private fun finishSpeakCallback() {
        val callback = activeDone
        activeDone = null
        callback?.invoke()
    }

    private fun applyProfile(engine: TextToSpeech, profile: VoiceTtsProfile) {
        engine.setSpeechRate(profile.speechRate)
        engine.setPitch(profile.pitch)
        Log.d(TAG, "TTS profile=${profile.id} rate=${profile.speechRate} pitch=${profile.pitch}")
    }

    private fun applyPreferredVoice(engine: TextToSpeech) {
        if (preferredVoiceApplied) return
        preferredVoiceApplied = true
        val selected = engine.voices
            ?.filter { it.locale.language == Locale.CHINESE.language }
            ?.maxByOrNull(::voiceScore)
            ?: return
        runCatching {
            engine.voice = selected
            Log.d(TAG, "TTS voice=${selected.name}")
        }.onFailure { error ->
            Log.w(TAG, "设置 TTS voice 失败: ${error.message}")
        }
    }

    private fun voiceScore(voice: Voice): Int {
        val name = voice.name.lowercase(Locale.ROOT)
        var score = 0
        if (voice.locale.country.equals(Locale.CHINA.country, ignoreCase = true)) score += 20
        if (voice.locale.script.equals(Locale.SIMPLIFIED_CHINESE.script, ignoreCase = true)) score += 10
        if (voice.isNetworkConnectionRequired) score -= 8
        if (name.contains("female") || name.contains("woman") || name.contains("girl")) score += 35
        if (name.contains("xia") || name.contains("xiao") || name.contains("mei")) score += 18
        if (name.contains("male") || name.contains("man")) score -= 30
        score += voice.quality / 10
        score -= voice.latency / 20
        return score
    }
}

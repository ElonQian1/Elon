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
    private var released = false
    private var pendingSpeech: PendingSpeech? = null
    private val completionLedger = VoiceSpeakerCompletionLedger()
    private var preferredVoiceApplied = false
    private val serverTtsPlayer = VoiceServerTtsPlayer(appContext)

    /** 上一条 utterance 是否还在播放（用于打断判断）。 */
    val isSpeaking: Boolean get() = tts?.isSpeaking == true || serverTtsPlayer.isSpeaking

    override fun onInit(status: Int) {
        if (status != TextToSpeech.SUCCESS) {
            Log.w(TAG, "TTS 初始化失败 status=$status")
            ready = false
            tts?.shutdown()
            tts = null
            failPendingSpeech()
            return
        }
        val engine = tts ?: run {
            failPendingSpeech()
            return
        }
        if (released) {
            engine.shutdown()
            tts = null
            failPendingSpeech()
            return
        }
        val result = engine.setLanguage(Locale.SIMPLIFIED_CHINESE)
        if (result == TextToSpeech.LANG_MISSING_DATA || result == TextToSpeech.LANG_NOT_SUPPORTED) {
            Log.w(TAG, "zh-CN 不可用，使用系统默认语言")
            engine.setLanguage(Locale.getDefault())
        }
        applyPreferredVoice(engine)
        applyProfile(engine, VoiceTtsEmotion.profileFor(""))
        engine.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {
                if (completionLedger.isActive(utteranceId)) Log.d(TAG, "TTS utterance started")
            }
            override fun onDone(utteranceId: String?) {
                finishSpeakCallback(utteranceId, succeeded = true)
            }
            @Deprecated("Deprecated in API 21", ReplaceWith("onError(utteranceId, errorCode)"))
            override fun onError(utteranceId: String?) {
                finishSpeakCallback(utteranceId, succeeded = false)
            }
            override fun onError(utteranceId: String?, errorCode: Int) {
                finishSpeakCallback(utteranceId, succeeded = false)
            }
        })
        ready = true
        val pending = pendingSpeech
        pendingSpeech = null
        pending?.let { speak(it.text, it.profile, it.onDone, it.voiceIdOverride, it.onError) }
        Log.d(TAG, "TTS 初始化成功")
    }

    /**
     * 朗读文本。若上一条还在朗读，立即打断并播放新内容（QUEUE_FLUSH）。
     * 超长文本截断到 [MAX_SPEAK_CHARS] 字符，避免朗读时间过长。
     */
    fun speak(
        text: String,
        profile: VoiceTtsProfile? = null,
        onDone: (() -> Unit)? = null,
        voiceIdOverride: String? = null,
        onError: (() -> Unit)? = null,
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
            if (engine == null || released) {
                (onError ?: onDone)?.invoke()
            } else {
                pendingSpeech = PendingSpeech(content, profile, onDone, voiceIdOverride, onError)
            }
            return
        }
        pendingSpeech = null
        completionLedger.cancel()
        serverTtsPlayer.stop()
        if (tts?.isSpeaking == true) tts?.stop()
        val resolvedProfile = profile ?: VoiceTtsEmotion.profileFor(content)
        val effectiveVoiceId = resolveVoiceId(voiceIdOverride)
        val requestId = UUID.randomUUID().toString()
        completionLedger.begin(requestId, onDone, onError)
        if (VoiceTtsVoiceCatalog.isSystemVoiceId(effectiveVoiceId)) {
            Log.d(TAG, "使用手机系统 TTS profile=${resolvedProfile.id}")
            speakWithSystem(engine, content, resolvedProfile, requestId)
            return
        }
        Log.d(TAG, "尝试服务器情绪 TTS profile=${resolvedProfile.id}")
        if (serverTtsPlayer.trySpeak(
                text = content,
                profile = resolvedProfile,
                voiceIdOverride = effectiveVoiceId,
                onDone = { finishSpeakCallback(requestId, succeeded = true) },
                onFallback = {
                    if (!completionLedger.isActive(requestId)) return@trySpeak
                    Log.w(TAG, "服务器情绪 TTS 不可用，降级系统 TTS profile=${resolvedProfile.id}")
                    speakWithSystem(engine, content, resolvedProfile, requestId)
                }
            )
        ) {
            return
        }
        Log.w(TAG, "服务器情绪 TTS 被跳过，降级系统 TTS profile=${resolvedProfile.id}")
        speakWithSystem(engine, content, resolvedProfile, requestId)
    }

    private fun resolveVoiceId(voiceIdOverride: String?): String =
        voiceIdOverride
            ?.trim()
            ?.takeIf(VoiceTtsVoiceCatalog::isKnownVoiceId)
            ?: VoiceTtsPreferences.getSelectedVoiceId(appContext)

    private fun speakWithSystem(
        engine: TextToSpeech,
        content: String,
        profile: VoiceTtsProfile,
        requestId: String,
    ) {
        applyProfile(engine, profile)
        val params = Bundle()
        val result = engine.speak(content, TextToSpeech.QUEUE_FLUSH, params, requestId)
        if (result == TextToSpeech.ERROR) finishSpeakCallback(requestId, succeeded = false)
    }

    /** 立即停止当前朗读（用户开始新一轮语音输入时调用）。 */
    fun stop() {
        pendingSpeech = null
        completionLedger.cancel()
        serverTtsPlayer.stop()
        if (tts?.isSpeaking == true) tts?.stop()
    }

    /** 释放资源（Activity onDestroy）。 */
    fun release() {
        released = true
        pendingSpeech = null
        completionLedger.cancel()
        serverTtsPlayer.release()
        tts?.stop()
        tts?.shutdown()
        tts = null
        ready = false
        preferredVoiceApplied = false
    }

    private fun finishSpeakCallback(requestId: String?, succeeded: Boolean) {
        if (completionLedger.complete(requestId, succeeded)) {
            Log.d(TAG, if (succeeded) "TTS utterance completed" else "TTS utterance failed")
        }
    }

    private fun failPendingSpeech() {
        val pending = pendingSpeech
        pendingSpeech = null
        pending?.fail()
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

    private data class PendingSpeech(
        val text: String,
        val profile: VoiceTtsProfile?,
        val onDone: (() -> Unit)?,
        val voiceIdOverride: String?,
        val onError: (() -> Unit)?,
    ) {
        fun fail() = (onError ?: onDone)?.invoke()
    }
}

internal class VoiceSpeakerCompletionLedger {
    private var active: Active? = null

    @Synchronized
    fun begin(requestId: String, onDone: (() -> Unit)?, onError: (() -> Unit)?) {
        active = Active(requestId, onDone, onError)
    }

    @Synchronized
    fun cancel() {
        active = null
    }

    @Synchronized
    fun isActive(requestId: String?): Boolean = requestId != null && active?.requestId == requestId

    fun complete(requestId: String?, succeeded: Boolean): Boolean {
        val completion = synchronized(this) {
            active?.takeIf { requestId != null && it.requestId == requestId }?.also { active = null }
        } ?: return false
        if (succeeded) completion.onDone?.invoke() else (completion.onError ?: completion.onDone)?.invoke()
        return true
    }

    private data class Active(
        val requestId: String,
        val onDone: (() -> Unit)?,
        val onError: (() -> Unit)?,
    )
}

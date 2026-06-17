package com.elon.app

import com.elon.chatvoice.ChatVoiceIds

internal const val VOICE_TTS_COMPARISON_PREVIEW_TEXT = "你好呀，我是你的 AI 助手。以后就让我陪你聊天吧。"

internal data class VoiceTtsVoiceOption(
    val id: String,
    val displayName: String,
    val description: String,
    val previewText: String = VOICE_TTS_COMPARISON_PREVIEW_TEXT,
    val usesServerTts: Boolean = true
)

internal object VoiceTtsVoiceCatalog {
    const val COMPARISON_PREVIEW_TEXT = VOICE_TTS_COMPARISON_PREVIEW_TEXT
    const val SYSTEM_TTS_VOICE_ID = ChatVoiceIds.ANDROID_SYSTEM_TTS

    val systemVoice: VoiceTtsVoiceOption = VoiceTtsVoiceOption(
        id = SYSTEM_TTS_VOICE_ID,
        displayName = "手机系统 TTS",
        description = "使用这台 Android 手机自带的语音引擎，离线/低延迟，声线由系统决定。",
        usesServerTts = false
    )

    /** 内置预设，作为服务器目录加载前/失败时的兜底。 */
    val presetVoices: List<VoiceTtsVoiceOption> = listOf(
        VoiceTtsVoiceOption(
            id = "female_warm",
            displayName = "温柔姐姐",
            description = "温柔、陪伴、安慰感强，适合日常聊天。"
        ),
        VoiceTtsVoiceOption(
            id = "female_bright",
            displayName = "元气女友",
            description = "活泼、明亮、开心感强，适合轻松互动。"
        ),
        VoiceTtsVoiceOption(
            id = "female_mature",
            displayName = "成熟秘书",
            description = "成熟、稳定、清晰，适合正式回复和长文本。"
        ),
        VoiceTtsVoiceOption(
            id = "female_cool",
            displayName = "冷淡女王",
            description = "冷静、克制、距离感强，适合冷淡和压抑情绪。"
        ),
        VoiceTtsVoiceOption(
            id = "female_sweet",
            displayName = "甜美陪伴",
            description = "甜美、亲近、撒娇感强，适合陪伴和恋爱感场景。"
        )
    )

    /**
     * 从服务器目录动态加载的声线列表；null 表示尚未加载，此时 [allVoices] 使用 [presetVoices]。
     * 只在主线程写入，读取可在任意线程。
     */
    @Volatile
    private var dynamicVoices: List<VoiceTtsVoiceOption>? = null

    /** 当前完整声线列表 = 系统 TTS + 动态服务器声线（或内置预设）。 */
    val allVoices: List<VoiceTtsVoiceOption>
        get() = listOf(systemVoice) + (dynamicVoices ?: presetVoices)

    /** 用服务器返回的声线列表更新动态目录。每次打开声线选择器时调用。 */
    fun updateFromServer(voices: List<VoiceTtsVoiceOption>) {
        if (voices.isNotEmpty()) {
            dynamicVoices = voices
        }
    }

    fun isSystemVoiceId(voiceId: String): Boolean =
        voiceId == SYSTEM_TTS_VOICE_ID

    /**
     * 判断 voiceId 是否有效：
     * 1. 系统 TTS 永远有效
     * 2. 在当前 [allVoices] 中存在
     * 3. 或者 id 以 "female_" 开头（服务器标准前缀），允许在目录未加载前通过校验
     */
    fun isKnownVoiceId(voiceId: String): Boolean =
        voiceId == SYSTEM_TTS_VOICE_ID ||
            allVoices.any { it.id == voiceId } ||
            voiceId.startsWith("female_")

    fun findById(voiceId: String): VoiceTtsVoiceOption =
        allVoices.firstOrNull { it.id == voiceId }
            ?: allVoices.first { it.id == VoiceTtsPreferences.DEFAULT_VOICE_ID }
}


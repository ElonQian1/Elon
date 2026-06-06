package com.elon.app

internal data class VoiceTtsVoiceOption(
    val id: String,
    val displayName: String,
    val description: String,
    val previewText: String
)

internal object VoiceTtsVoiceCatalog {
    val presetVoices: List<VoiceTtsVoiceOption> = listOf(
        VoiceTtsVoiceOption(
            id = "female_warm",
            displayName = "温柔姐姐",
            description = "温柔、陪伴、安慰感强，适合日常聊天。",
            previewText = "别担心，我会一直陪着你，慢慢来就好。"
        ),
        VoiceTtsVoiceOption(
            id = "female_bright",
            displayName = "元气女友",
            description = "活泼、明亮、开心感强，适合轻松互动。",
            previewText = "太好了！今天也要一起加油呀！"
        ),
        VoiceTtsVoiceOption(
            id = "female_mature",
            displayName = "成熟秘书",
            description = "成熟、稳定、清晰，适合正式回复和长文本。",
            previewText = "我已经帮你整理好了，接下来我们一步一步处理。"
        ),
        VoiceTtsVoiceOption(
            id = "female_cool",
            displayName = "冷淡女王",
            description = "冷静、克制、距离感强，适合冷淡和压抑情绪。",
            previewText = "我知道了。既然你已经决定，那就按你的想法来。"
        ),
        VoiceTtsVoiceOption(
            id = "female_sweet",
            displayName = "甜美陪伴",
            description = "甜美、亲近、撒娇感强，适合陪伴和恋爱感场景。",
            previewText = "你终于回来啦，我刚刚一直在等你呢。"
        )
    )

    fun isKnownVoiceId(voiceId: String): Boolean =
        presetVoices.any { it.id == voiceId }

    fun findById(voiceId: String): VoiceTtsVoiceOption =
        presetVoices.firstOrNull { it.id == voiceId }
            ?: presetVoices.first { it.id == VoiceTtsPreferences.DEFAULT_VOICE_ID }
}

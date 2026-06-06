package com.elon.app

import java.util.Locale

internal data class VoiceTtsProfile(
    val id: String,
    val speechRate: Float,
    val pitch: Float,
    // Final server voice is overridden by the user's VoiceTtsPreferences choice.
    val serverVoiceId: String,
    val serverEmotionId: String,
    val serverIntensity: String
)

internal object VoiceTtsEmotion {
    private val neutral = VoiceTtsProfile(
        "warm_neutral",
        speechRate = 1.02f,
        pitch = 1.04f,
        serverVoiceId = "female_warm",
        serverEmotionId = "normal",
        serverIntensity = "normal"
    )
    private val gentle = VoiceTtsProfile(
        "gentle_comfort",
        speechRate = 0.94f,
        pitch = 1.03f,
        serverVoiceId = "female_warm",
        serverEmotionId = "gentle_comfort",
        serverIntensity = "immersive"
    )
    private val bright = VoiceTtsProfile(
        "bright_happy",
        speechRate = 1.08f,
        pitch = 1.10f,
        serverVoiceId = "female_bright",
        serverEmotionId = "happy_sweet",
        serverIntensity = "immersive"
    )
    private val softSad = VoiceTtsProfile(
        "soft_low",
        speechRate = 0.90f,
        pitch = 0.96f,
        serverVoiceId = "female_warm",
        serverEmotionId = "wronged_crying",
        serverIntensity = "immersive"
    )
    private val calmSerious = VoiceTtsProfile(
        "calm_serious",
        speechRate = 0.98f,
        pitch = 0.99f,
        serverVoiceId = "female_mature",
        serverEmotionId = "serious_encourage",
        serverIntensity = "normal"
    )
    private val whisper = VoiceTtsProfile(
        "low_whisper",
        speechRate = 0.88f,
        pitch = 0.94f,
        serverVoiceId = "female_warm",
        serverEmotionId = "whisper_low",
        serverIntensity = "immersive"
    )

    fun profileFor(text: String): VoiceTtsProfile {
        val content = text.trim()
        if (content.isEmpty()) return neutral
        val lower = content.lowercase(Locale.ROOT)
        val exclamationCount = content.count { it == '!' || it == '！' }

        return when {
            lower.hasAny(whisperWords) -> whisper
            content.hasAny(sadWords) -> softSad
            content.hasAny(comfortWords) -> gentle
            exclamationCount >= 2 || content.hasAny(happyWords) -> bright
            content.hasAny(seriousWords) -> calmSerious
            else -> neutral
        }
    }

    private fun String.hasAny(words: List<String>): Boolean =
        words.any { contains(it, ignoreCase = true) }

    private val comfortWords = listOf(
        "我在",
        "别怕",
        "慢慢来",
        "没关系",
        "不用急",
        "辛苦了",
        "陪着你",
        "抱抱",
        "放心",
        "可以的"
    )

    private val happyWords = listOf(
        "太好了",
        "真棒",
        "恭喜",
        "开心",
        "好呀",
        "当然可以",
        "没问题",
        "哈哈",
        "呀",
        "啦"
    )

    private val sadWords = listOf(
        "难过",
        "委屈",
        "想哭",
        "哭",
        "失落",
        "遗憾",
        "对不起",
        "抱歉",
        "心疼",
        "可惜"
    )

    private val seriousWords = listOf(
        "不能",
        "不要",
        "风险",
        "失败",
        "错误",
        "异常",
        "需要检查",
        "涉及项目开发",
        "请到「项目」",
        "请到项目"
    )

    private val whisperWords = listOf(
        "小声",
        "悄悄",
        "低声",
        "耳语",
        "轻声"
    )
}

package com.elon.chatvoice

object ChatVoiceIds {
    const val ANDROID_SYSTEM_TTS = "android_system"
    const val TARGET_TRANSCRIBE_ONLY = "transcribe_only"
    const val TARGET_EXTERNAL_GROUP = "external_group"
    const val FB2_DEFAULT_GROUP_ID = "ext_fb2_official"
}

data class ChatVoiceConfig(
    val baseUrl: String,
    val bearerTokenProvider: () -> String?,
    val defaultGroupId: String = ChatVoiceIds.FB2_DEFAULT_GROUP_ID,
    val appId: String = "fb2",
    val preferServerAsr: Boolean = true,
    val preferServerTts: Boolean = true,
    val fallbackToSystemAsr: Boolean = true,
    val fallbackToSystemTts: Boolean = true,
    val selectedTtsVoiceProvider: () -> String? = { null },
) {
    val normalizedBaseUrl: String
        get() = baseUrl.trim().trimEnd('/')
}

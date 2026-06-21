package com.elon.chatvoice

import org.json.JSONObject

/**
 * Public helpers for wiring the main-project chat-bootstrap response into the
 * reusable Android composer. Hosts should prefer these presets over rebuilding
 * the ASR/TTS fallback chain by hand.
 */
object VoiceComposerBootstrap {
    fun fb2GroupChatConfig(
        baseUrl: String,
        bearerTokenProvider: () -> String?,
        bootstrapJson: String? = null,
        eventSink: ChatVoiceEventSink? = null,
        copy: VoiceComposerCopy = VoiceComposerCopy(),
        style: VoiceComposerStyle = VoiceComposerStyle(),
    ): VoiceComposerConfig {
        val defaults = parseDefaults(bootstrapJson)
        val serverConfig = ChatVoiceConfig(
            baseUrl = baseUrl,
            bearerTokenProvider = bearerTokenProvider,
            defaultGroupId = ChatVoiceIds.FB2_DEFAULT_GROUP_ID,
            appId = "fb2",
            preferServerAsr = false,
            preferServerTts = true,
            fallbackToSystemAsr = true,
            fallbackToSystemTts = true,
        )
        return VoiceComposerConfig(
            chatMode = defaults.chatMode,
            releaseZone = defaults.releaseZone,
            sendZoneSendsVoice = true,
            recordingOverlayEnabled = defaults.recordingOverlayEnabled,
            languageTag = defaults.languageTag,
            preferOfflineAsr = defaults.preferOfflineAsr,
            asr = VoiceComposerAsrConfig(
                serverFallbackEnabled = defaults.serverFallbackEnabled,
                serverConfig = serverConfig,
                serverOptions = ServerAsrOptions(language = "auto"),
                localStartTimeoutMs = defaults.localStartTimeoutMs,
                localResultTimeoutMs = defaults.localResultTimeoutMs,
                localEngineFallbackEnabled = defaults.localEngineFallbackEnabled,
                prewarmLocalEngine = defaults.prewarmLocalEngine,
                deleteRecordedFileAfterResult = true,
            ),
            copy = copy,
            style = style,
            eventSink = eventSink,
        )
    }

    fun applyFb2GroupChatConfig(
        composer: VoiceComposerView,
        baseUrl: String,
        bearerTokenProvider: () -> String?,
        bootstrapJson: String? = null,
        eventSink: ChatVoiceEventSink? = null,
        copy: VoiceComposerCopy = VoiceComposerCopy(),
        style: VoiceComposerStyle = VoiceComposerStyle(),
    ): VoiceComposerConfig {
        val config = fb2GroupChatConfig(
            baseUrl = baseUrl,
            bearerTokenProvider = bearerTokenProvider,
            bootstrapJson = bootstrapJson,
            eventSink = eventSink,
            copy = copy,
            style = style,
        )
        composer.applyConfig(config)
        return config
    }

    private fun parseDefaults(bootstrapJson: String?): BootstrapVoiceDefaults {
        if (bootstrapJson.isNullOrBlank()) return BootstrapVoiceDefaults()
        val root = runCatching { JSONObject(bootstrapJson) }.getOrNull() ?: return BootstrapVoiceDefaults()
        val defaultConfig = root.optJSONObject("voice")
            ?.optJSONObject("composer")
            ?.optJSONObject("defaultConfig")
            ?: root.optJSONObject("composer")?.optJSONObject("defaultConfig")
            ?: return BootstrapVoiceDefaults()
        val asr = defaultConfig.optJSONObject("asr")
        return BootstrapVoiceDefaults(
            chatMode = parseChatMode(defaultConfig.optString("chatMode")),
            releaseZone = parseZone(defaultConfig.optString("releaseZone")),
            recordingOverlayEnabled = defaultConfig.optBoolean("recordingOverlayEnabled", true),
            languageTag = defaultConfig.optString("languageTag").ifBlank { "zh-CN" },
            preferOfflineAsr = defaultConfig.optBoolean("preferOfflineAsr", false),
            serverFallbackEnabled = asr?.optBoolean("serverFallbackEnabled", true) ?: true,
            localStartTimeoutMs = asr?.optLong(
                "localStartTimeoutMs",
                SystemSpeechTranscriber.DEFAULT_START_TIMEOUT_MS,
            ) ?: SystemSpeechTranscriber.DEFAULT_START_TIMEOUT_MS,
            localResultTimeoutMs = asr?.optLong("localResultTimeoutMs", 4_500L) ?: 4_500L,
            localEngineFallbackEnabled = asr?.optBoolean("localEngineFallbackEnabled", true) ?: true,
            prewarmLocalEngine = asr?.optBoolean("prewarmLocalEngine", true) ?: true,
        )
    }

    private fun parseChatMode(value: String): ChatVoiceMode =
        runCatching { ChatVoiceMode.valueOf(value.ifBlank { ChatVoiceMode.FRIEND_CHAT.name }) }
            .getOrDefault(ChatVoiceMode.FRIEND_CHAT)

    private fun parseZone(value: String): ChatVoiceZone =
        runCatching { ChatVoiceZone.valueOf(value.ifBlank { ChatVoiceZone.SEND.name }) }
            .getOrDefault(ChatVoiceZone.SEND)
}

private data class BootstrapVoiceDefaults(
    val chatMode: ChatVoiceMode = ChatVoiceMode.FRIEND_CHAT,
    val releaseZone: ChatVoiceZone = ChatVoiceZone.SEND,
    val recordingOverlayEnabled: Boolean = true,
    val languageTag: String = "zh-CN",
    val preferOfflineAsr: Boolean = false,
    val serverFallbackEnabled: Boolean = true,
    val localStartTimeoutMs: Long = SystemSpeechTranscriber.DEFAULT_START_TIMEOUT_MS,
    val localResultTimeoutMs: Long = 4_500L,
    val localEngineFallbackEnabled: Boolean = true,
    val prewarmLocalEngine: Boolean = true,
)

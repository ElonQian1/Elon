package com.elon.app

internal data class WebChatProductionComposerCommand(
    val action: String,
    val label: String,
    val nativeSelector: String,
)

internal object WebChatProductionComposerCommandCatalog {
    fun resolve(
        provider: WebChatProviderIdentity,
        streaming: Boolean,
        dictationActive: Boolean,
    ): List<WebChatProductionComposerCommand> {
        val commands = mutableListOf<WebChatProductionComposerCommand>()
        if (provider.supports(WebChatProviderCapability.STOP_GENERATION) && streaming) {
            commands += command(provider, "chatgpt_stop_generation", "停止生成", "stop-generation")
        }
        if (provider.supports(WebChatProviderCapability.DICTATION) && !streaming) {
            commands += if (dictationActive) {
                command(provider, "chatgpt_submit_dictation", "完成网页听写", "submit-dictation")
            } else {
                command(provider, "chatgpt_start_dictation", "网页听写", "start-dictation")
            }
        }
        if (
            provider.supports(WebChatProviderCapability.REALTIME_VOICE) &&
            !streaming &&
            !dictationActive
        ) {
            commands += command(
                provider,
                "chatgpt_start_realtime_voice",
                "实时语音",
                "start-realtime-voice",
            )
        }
        return commands
    }

    private fun command(
        provider: WebChatProviderIdentity,
        action: String,
        label: String,
        selectorSuffix: String,
    ) = WebChatProductionComposerCommand(
        action = action,
        label = label,
        nativeSelector = "web-chat-composer-command:${provider.id.wireValue}:$selectorSuffix",
    )
}

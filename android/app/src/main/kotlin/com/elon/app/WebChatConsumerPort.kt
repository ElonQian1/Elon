package com.elon.app

internal data class WebChatConsumerOption(
    val id: String,
    val label: String,
    val selected: Boolean,
    val semantic: String,
    val opensSubmenu: Boolean,
    val nativeSelector: String,
)

internal data class WebChatConsumerState(
    val streaming: Boolean,
    val dictationActive: Boolean,
    val composerSections: Map<String, List<WebChatConsumerOption>>,
)

internal data class WebChatConsumerCommandResult(
    val accepted: Boolean,
    val error: String? = null,
    val requestId: String? = null,
)

internal interface WebChatConsumerPort {
    fun state(): WebChatConsumerState
    fun requestComposerOptions(section: String): WebChatConsumerCommandResult
    fun selectComposerOption(section: String, optionId: String): WebChatConsumerCommandResult
    fun executeSessionCommand(action: String): WebChatConsumerCommandResult
}

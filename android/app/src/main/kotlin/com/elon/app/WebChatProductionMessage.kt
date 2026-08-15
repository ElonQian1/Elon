package com.elon.app

enum class WebChatMessageAction(val wireValue: String) {
    COPY("copy"),
    REGENERATE("regenerate"),
    MORE("more"),
}

data class WebChatProductionContentPart(
    val type: String,
    val label: String,
    val language: String? = null,
    val mediaType: String? = null,
    val targetHost: String? = null,
    val lineCount: Int? = null,
    val rowCount: Int? = null,
    val columnCount: Int? = null,
)

data class WebChatProductionMessage(
    val providerWireValue: String,
    val sourceMessageId: String,
    val actions: Set<WebChatMessageAction>,
    val renderMarkdown: Boolean = false,
    val contentParts: List<WebChatProductionContentPart> = emptyList(),
)

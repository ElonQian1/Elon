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
    val assetHandle: String? = null,
    val imageSource: String? = null,
    val imageWidth: Int? = null,
    val imageHeight: Int? = null,
    val previewPending: Boolean = false,
    val lineCount: Int? = null,
    val rowCount: Int? = null,
    val columnCount: Int? = null,
    val richCard: WebChatProductionRichCard? = null,
)

data class WebChatProductionMessage(
    val providerWireValue: String,
    val sourceMessageId: String,
    val actions: Set<WebChatMessageAction>,
    val renderMarkdown: Boolean = false,
    val contentParts: List<WebChatProductionContentPart> = emptyList(),
)

package com.elon.app

/** Keeps native-only message actions available even when the official control scan is stale. */
internal object WebChatProductionMessageActionPolicy {
    fun resolve(message: ChatMessage): Set<WebChatMessageAction> {
        val metadata = message.webChatMessage ?: return emptySet()
        return buildSet {
            addAll(metadata.actions)
            if (message.content.isNotBlank()) add(WebChatMessageAction.COPY)
            if (
                metadata.providerWireValue == WebChatProviderId.CHATGPT_WEB.wireValue &&
                message.role == "friend" &&
                message.content.isNotBlank() &&
                metadata.sourceMessageId.isNotBlank()
            ) {
                add(WebChatMessageAction.MORE)
            }
        }
    }
}

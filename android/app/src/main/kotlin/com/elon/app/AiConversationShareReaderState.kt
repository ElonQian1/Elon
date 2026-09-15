package com.elon.app

internal data class AiConversationShareReaderPosition(val messageId: String, val offset: Int)

internal class AiConversationShareReaderSearch(private val messages: List<ChatMessage>) {
    var query: String = ""
        private set
    var matches: List<Int> = emptyList()
        private set
    var current: Int = -1
        private set

    fun update(value: String): Int? {
        query = value.trim()
        matches = if (query.isEmpty()) emptyList() else messages.indices.filter { index ->
            val message = messages[index]
            message.role != "ai-conversation-share-gap" && sequenceOf(message.content, message.senderLabel.orEmpty(),
                message.attachments.orEmpty().joinToString("\n") { it.displayName.orEmpty() + "\n" + it.transcription.orEmpty() },
                message.webChatMessage?.contentParts.orEmpty().joinToString("\n") {
                    it.label + "\n" + it.textBlock?.content.orEmpty() + "\n" + it.richCard?.title.orEmpty()
                }).any { it.contains(query, ignoreCase = true) }
        }
        current = if (matches.isEmpty()) -1 else 0
        return matches.getOrNull(current)
    }

    fun move(delta: Int): Int? {
        if (matches.isEmpty()) return null
        current = Math.floorMod(current + delta, matches.size)
        return matches[current]
    }
}

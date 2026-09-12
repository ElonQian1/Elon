package com.elon.app.chatgptweb

/** Enriches an exact DOM body match, never resurrects an absent or changed block. */
internal object WebChatTextBlockContinuity {
    fun merge(previous: ChatGptWebMessage?, incoming: ChatGptWebMessage): ChatGptWebMessage {
        if (previous == null || previous.id.isBlank() || previous.id != incoming.id ||
            previous.role != incoming.role || previous.state != "completed" || incoming.state != "completed"
        ) return incoming
        val oldBlocks = previous.parts.filter { it.textBlock?.complete == true }
        val currentBlocks = incoming.parts.mapNotNull { it.textBlock }
        if (oldBlocks.isEmpty()) return incoming
        return incoming.copy(parts = incoming.parts.map { part ->
            val block = part.textBlock ?: return@map part
            if (part.type != "code" || !block.complete || block.title.isNotEmpty() ||
                block.sourceMessageId != null || currentBlocks.count { it.content == block.content } != 1
            ) return@map part
            val old = oldBlocks.singleOrNull { it.textBlock?.content == block.content } ?: return@map part
            val known = old.textBlock ?: return@map part
            if (block.language.isNotEmpty() && block.language != known.language) return@map part
            old
        })
    }
}

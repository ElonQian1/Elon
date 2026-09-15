package com.elon.app.chatgptweb

internal object WebChatSnapshotWindowMerger {
    fun mergeConversation(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        authoritativeHistory: Boolean,
    ): ChatGptWebSnapshot {
        val previousIdentity = ChatGptWebConversationPath.fromUrl(previous?.url)
            ?.let(ChatGptWebConversationPath::identity)
        val incomingIdentity = ChatGptWebConversationPath.fromUrl(incoming.url)
            ?.let(ChatGptWebConversationPath::identity)
        return merge(previous, incoming,
            previousIdentity != null && previousIdentity == incomingIdentity, authoritativeHistory)
    }

    fun merge(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        sameConversation: Boolean,
        authoritativeHistory: Boolean = false,
    ): ChatGptWebSnapshot {
        if (!sameConversation || previous == null) return incoming
        // Only a complete private history read may discard an old branch or inflated window.
        if (authoritativeHistory && !previous.streaming && incoming.messages.isNotEmpty() &&
            incoming.messageWindowStart == 0 && incoming.observedMessageCount == incoming.messages.size
        ) {
            val bounded = incoming.messages.takeLast(MAX_MESSAGES)
            return incoming.copy(messages = bounded, messageWindowStart = incoming.messages.size - bounded.size)
        }
        if (incoming.messages.isEmpty()) {
            return incoming.copy(
                messages = previous.messages,
                messageWindowStart = previous.messageWindowStart,
                observedMessageCount = maxOf(
                    previous.observedMessageCount,
                    incoming.observedMessageCount,
                ),
            )
        }
        val merged = mergeByStableId(previous.messages, incoming.messages)
            ?: mergeByWindow(previous, incoming)
        val observedCount = maxOf(
            previous.observedMessageCount,
            incoming.observedMessageCount,
            previous.messageWindowStart + previous.messages.size,
            incoming.messageWindowStart + incoming.messages.size,
            merged.size,
        )
        val bounded = merged.takeLast(MAX_MESSAGES)
        return incoming.copy(
            messages = bounded,
            messageWindowStart = (observedCount - bounded.size).coerceAtLeast(0),
            observedMessageCount = observedCount,
        )
    }

    private fun mergeByStableId(
        previous: List<ChatGptWebMessage>,
        incoming: List<ChatGptWebMessage>,
    ): List<ChatGptWebMessage>? {
        val previousIndex = previous.mapIndexedNotNull { index, message ->
            messageKey(message)?.let { it to index }
        }.toMap()
        val common = incoming.mapIndexedNotNull { incomingIndex, message ->
            previousIndex[messageKey(message)]?.let { previousPosition ->
                Match(previousPosition, incomingIndex)
            }
        }
        if (common.isEmpty() || common.zipWithNext().any { (a, b) -> a.previous >= b.previous }) {
            return null
        }
        val first = common.first()
        val last = common.last()
        val enriched = incoming.map { message ->
            val old = previousIndex[messageKey(message)]?.let(previous::get)
            val current = if (old != null && isStreamAlias(message) && !isStreamAlias(old)) {
                message.copy(id = old.id)
            } else message
            WebChatTextBlockContinuity.merge(old?.copy(id = current.id), current)
        }
        if (common.size == incoming.size) {
            // Missing rows in a known, ordered DOM subset are not deleted provider messages.
            val updates = enriched.associateBy(::messageKey)
            return deduplicated(previous.map { updates[messageKey(it)] ?: it })
        }
        return deduplicated(previous.take(first.previous) + enriched + previous.drop(last.previous + 1))
    }

    private fun mergeByWindow(
        previous: ChatGptWebSnapshot,
        incoming: ChatGptWebSnapshot,
    ): List<ChatGptWebMessage> {
        val indexed = sortedMapOf<Int, ChatGptWebMessage>()
        previous.messages.forEachIndexed { index, message ->
            indexed[previous.messageWindowStart + index] = message
        }
        incoming.messages.forEachIndexed { index, message ->
            val position = incoming.messageWindowStart + index
            indexed[position] = WebChatTextBlockContinuity.merge(indexed[position], message)
        }
        val last = indexed.lastKey()
        var first = last
        while (indexed.containsKey(first - 1)) first--
        return deduplicated((first..last).mapNotNull(indexed::get))
    }

    private fun deduplicated(messages: List<ChatGptWebMessage>): List<ChatGptWebMessage> {
        val positions = mutableMapOf<String, Int>()
        val result = mutableListOf<ChatGptWebMessage>()
        messages.forEach { message ->
            val key = messageKey(message)
            val position = key?.let(positions::get)
            if (position == null) {
                if (key != null) positions[key] = result.size
                result.add(message)
            } else if (isStreamAlias(result[position]) && !isStreamAlias(message)) {
                result[position] = message
            }
        }
        return result
    }

    // A stream placeholder carries the exact provider message UUID. Text is
    // deliberately not used: separate turns can have identical answers.
    private fun messageKey(message: ChatGptWebMessage): String? =
        message.id.takeIf(String::isNotBlank)?.let { id ->
            message.role + ":" + if (isStreamAlias(message)) id.removePrefix("private-stream:") else id
        }

    private fun isStreamAlias(message: ChatGptWebMessage): Boolean =
        message.role == "assistant" && STREAM_ALIAS.matches(message.id)

    private data class Match(val previous: Int, val incoming: Int)

    private const val MAX_MESSAGES = 80
    private val STREAM_ALIAS = Regex("private-stream:[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}")
}

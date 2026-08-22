package com.elon.app.chatgptweb

internal object WebChatSnapshotWindowMerger {
    fun merge(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        sameConversation: Boolean,
    ): ChatGptWebSnapshot {
        if (!sameConversation || previous == null) return incoming
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
            message.id.takeIf(String::isNotBlank)?.let { it to index }
        }.toMap()
        val common = incoming.mapIndexedNotNull { incomingIndex, message ->
            previousIndex[message.id]?.let { previousPosition ->
                Match(previousPosition, incomingIndex)
            }
        }
        if (common.isEmpty() || common.zipWithNext().any { (a, b) -> a.previous >= b.previous }) {
            return null
        }
        val first = common.first()
        val last = common.last()
        return deduplicated(
            previous.take(first.previous) + incoming + previous.drop(last.previous + 1),
        )
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
            indexed[incoming.messageWindowStart + index] = message
        }
        val last = indexed.lastKey()
        var first = last
        while (indexed.containsKey(first - 1)) first--
        return deduplicated((first..last).mapNotNull(indexed::get))
    }

    private fun deduplicated(messages: List<ChatGptWebMessage>): List<ChatGptWebMessage> {
        val seen = linkedSetOf<String>()
        return messages.filter { message ->
            val id = message.id.takeIf(String::isNotBlank) ?: return@filter true
            seen.add(id)
        }
    }

    private data class Match(val previous: Int, val incoming: Int)

    private const val MAX_MESSAGES = 80
}

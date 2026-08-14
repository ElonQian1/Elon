package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal object GoogleWebSnapshotMerger {
    fun merge(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        sameConversation: Boolean,
    ): ChatGptWebSnapshot {
        if (!sameConversation || previous == null) return stabilized(incoming)
        val currentUser = incoming.messages.lastOrNull { it.role == "user" }
            ?: return incoming.copy(
                messages = previous.messages,
                messageWindowStart = previous.messageWindowStart,
                observedMessageCount = previous.observedMessageCount,
            )
        val previousUserIndex = previous.messages.indexOfLast { it.role == "user" }
        val sameTurn = previousUserIndex >= 0 &&
            normalized(previous.messages[previousUserIndex].content) == normalized(currentUser.content)
        val prefix = if (sameTurn) {
            previous.messages.take(previousUserIndex)
        } else {
            previous.messages
        }
        val currentAssistant = incoming.messages.lastOrNull { it.role == "assistant" }
        val merged = buildList {
            addAll(prefix)
            add(currentUser)
            currentAssistant?.let(::add)
        }
        return incoming.withBoundedMessages(merged, previous.messageWindowStart)
    }

    private fun stabilized(snapshot: ChatGptWebSnapshot): ChatGptWebSnapshot {
        return snapshot.withBoundedMessages(snapshot.messages, snapshot.messageWindowStart)
    }

    private fun stabilizeMessages(
        messages: List<ChatGptWebMessage>,
        baseWindowStart: Int,
    ): List<ChatGptWebMessage> = messages.mapIndexed { index, message ->
        message.copy(
            id = "google-message-${baseWindowStart + index}-${message.role}",
        )
    }

    private fun ChatGptWebSnapshot.withBoundedMessages(
        source: List<ChatGptWebMessage>,
        baseWindowStart: Int,
    ): ChatGptWebSnapshot {
        val stabilized = stabilizeMessages(source, baseWindowStart)
        val bounded = stabilized.takeLast(MAX_MESSAGES)
        val dropped = stabilized.size - bounded.size
        return copy(
            messages = bounded,
            messageWindowStart = baseWindowStart + dropped,
            observedMessageCount = baseWindowStart + stabilized.size,
        )
    }

    private fun normalized(value: String): String = value.trim().replace(WHITESPACE, " ")

    private val WHITESPACE = Regex("\\s+")
    private const val MAX_MESSAGES = 32
}

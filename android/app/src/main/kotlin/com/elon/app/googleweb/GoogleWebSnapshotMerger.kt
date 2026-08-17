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
        if (incoming.messages.isEmpty()) {
            return incoming.copy(
                messages = previous.messages,
                messageWindowStart = previous.messageWindowStart,
                observedMessageCount = previous.observedMessageCount,
            )
        }
        val mergedTurns = messageTurns(previous.messages).toMutableList()
        messageTurns(incoming.messages).forEach { incomingTurn ->
            val userText = incomingTurn.firstOrNull()?.content?.let(::normalized).orEmpty()
            if (userText.isEmpty()) return@forEach
            val existingIndex = mergedTurns.indexOfFirst { turn ->
                turn.firstOrNull()?.content?.let(::normalized) == userText
            }
            if (existingIndex < 0) {
                mergedTurns += incomingTurn
            } else if (incomingTurn.any { it.role == "assistant" }) {
                mergedTurns[existingIndex] = incomingTurn
            }
        }
        val merged = mergedTurns.flatten()
        return incoming.withBoundedMessages(merged, previous.messageWindowStart)
    }

    private fun messageTurns(messages: List<ChatGptWebMessage>): List<List<ChatGptWebMessage>> {
        val turns = mutableListOf<MutableList<ChatGptWebMessage>>()
        messages.forEach { message ->
            if (message.role == "user") {
                turns += mutableListOf(message)
            } else {
                turns.lastOrNull()?.add(message)
            }
        }
        return turns
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
    private const val MAX_MESSAGES = 80
}

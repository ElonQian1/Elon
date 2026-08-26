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
            val preserved = sanitizeCached(previous)
            return incoming.copy(
                messages = preserved.messages,
                messageWindowStart = preserved.messageWindowStart,
                observedMessageCount = preserved.observedMessageCount,
            )
        }
        val mergedTurns = stabilizedTurns(previous.messages).toMutableList()
        messageTurns(incoming.messages).forEach { incomingTurn ->
            val userText = incomingTurn.firstOrNull()?.content?.let(::normalized).orEmpty()
            if (userText.isEmpty()) return@forEach
            val existingIndex = mergedTurns.indexOfFirst { turn ->
                turn.firstOrNull()?.content?.let(::normalized) == userText
            }
            val admittedTurn = admitTurn(
                mergedTurns = mergedTurns,
                incomingTurn = incomingTurn,
                existingIndex = existingIndex,
                completedAfterStreaming = previous.streaming && !incoming.streaming,
            )
            if (existingIndex < 0) {
                mergedTurns += admittedTurn
            } else if (admittedTurn.any { it.role == "assistant" }) {
                mergedTurns[existingIndex] = admittedTurn
            }
        }
        val merged = mergedTurns.flatten()
        return incoming.withBoundedMessages(merged, previous.messageWindowStart)
    }

    fun sanitizeCached(snapshot: ChatGptWebSnapshot): ChatGptWebSnapshot =
        snapshot.withBoundedMessages(
            stabilizedTurns(snapshot.messages).flatten(),
            snapshot.messageWindowStart,
        )

    private fun admitTurn(
        mergedTurns: List<List<ChatGptWebMessage>>,
        incomingTurn: List<ChatGptWebMessage>,
        existingIndex: Int,
        completedAfterStreaming: Boolean,
    ): List<ChatGptWebMessage> {
        val assistantFingerprint = assistantFingerprint(incomingTurn) ?: return incomingTurn
        val duplicatesEarlierAnswer = mergedTurns.withIndex().any { (index, turn) ->
            index != existingIndex && assistantFingerprint(turn) == assistantFingerprint
        }
        if (!duplicatesEarlierAnswer) return incomingTurn

        val existingTurnAwaitingAnswer = existingIndex >= 0 &&
            mergedTurns[existingIndex].none { it.role == "assistant" }
        if (existingTurnAwaitingAnswer && completedAfterStreaming) return incomingTurn
        return incomingTurn.filterNot { it.role == "assistant" }
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

    private fun stabilizedTurns(messages: List<ChatGptWebMessage>): List<List<ChatGptWebMessage>> {
        val turns = messageTurns(messages).map { it.toMutableList() }
        for (index in 1 until turns.lastIndex) {
            val previousFingerprint = assistantFingerprint(turns[index - 1]) ?: continue
            if (assistantFingerprint(turns[index]) != previousFingerprint) continue
            turns[index].removeAll { it.role == "assistant" }
        }
        return turns
    }

    private fun assistantFingerprint(turn: List<ChatGptWebMessage>): String? {
        val content = turn.asSequence()
            .filter { it.role == "assistant" }
            .map { normalized(it.content) }
            .filter(String::isNotEmpty)
            .toList()
        return content.takeIf(List<String>::isNotEmpty)?.joinToString("\u0000")
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

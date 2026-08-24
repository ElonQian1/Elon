package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal class WebChatRealtimeVoiceTranscriptContinuity {
    private enum class Phase {
        IDLE,
        ACTIVE,
        RECOVERING,
    }

    private var phase = Phase.IDLE
    private var conversationPath: String? = null
    private var retainedSnapshot: ChatGptWebSnapshot? = null

    fun begin(current: ChatGptWebSnapshot?) {
        phase = Phase.ACTIVE
        conversationPath = ChatGptWebConversationPath.fromUrl(current?.url)
        retainedSnapshot = current?.takeIf(::isConversationTranscript)
    }

    fun end(current: ChatGptWebSnapshot?): ChatGptWebSnapshot? {
        retain(current)
        phase = Phase.RECOVERING
        return retainedSnapshot
    }

    fun resolve(incoming: ChatGptWebSnapshot): ChatGptWebSnapshot? = when (phase) {
        Phase.IDLE -> incoming
        Phase.ACTIVE -> {
            retain(incoming)
            null
        }
        Phase.RECOVERING -> resolveRecovery(incoming)
    }

    fun reset() {
        phase = Phase.IDLE
        conversationPath = null
        retainedSnapshot = null
    }

    private fun resolveRecovery(incoming: ChatGptWebSnapshot): ChatGptWebSnapshot? {
        if (!isConversationTranscript(incoming)) {
            return retainedSnapshot
        }
        val incomingPath = ChatGptWebConversationPath.fromUrl(incoming.url)
        if (conversationPath != null && incomingPath != conversationPath) {
            reset()
            return incoming
        }
        retain(incoming)
        return retainedSnapshot
    }

    private fun retain(candidate: ChatGptWebSnapshot?) {
        if (candidate == null || !isConversationTranscript(candidate)) return
        val candidatePath = ChatGptWebConversationPath.fromUrl(candidate.url) ?: return
        if (conversationPath != null && candidatePath != conversationPath) return
        if (conversationPath == null) conversationPath = candidatePath

        val retained = retainedSnapshot
        retainedSnapshot = retained?.let {
            val retainedEnd = it.messageWindowStart + it.messages.size
            val candidateEnd = candidate.messageWindowStart + candidate.messages.size
            if (candidateEnd < retainedEnd) mergeStatus(it, candidate) else mergeSnapshots(it, candidate)
        } ?: candidate
    }

    private fun mergeStatus(
        retained: ChatGptWebSnapshot,
        incoming: ChatGptWebSnapshot,
    ): ChatGptWebSnapshot = retained.copy(
        authenticated = incoming.authenticated,
        composerReady = incoming.composerReady,
        streaming = incoming.streaming,
        currentModel = incoming.currentModel.ifBlank { retained.currentModel },
        capabilities = incoming.capabilities,
        loginRequired = incoming.loginRequired,
    )

    private fun mergeSnapshots(
        retained: ChatGptWebSnapshot,
        incoming: ChatGptWebSnapshot,
    ): ChatGptWebSnapshot {
        val incomingById = incoming.messages.associateBy { it.id }
        val retainedIds = retained.messages.mapTo(mutableSetOf()) { it.id }
        val messages = buildList {
            retained.messages.forEach { previous ->
                add(incomingById[previous.id]?.let { mergeMessage(previous, it) } ?: previous)
            }
            incoming.messages.filterNot { it.id in retainedIds }.forEach(::add)
        }
        return incoming.copy(
            messages = messages,
            messageWindowStart = minOf(retained.messageWindowStart, incoming.messageWindowStart),
            observedMessageCount = maxOf(
                retained.observedMessageCount,
                incoming.observedMessageCount,
                messages.size,
            ),
            currentModel = incoming.currentModel.ifBlank { retained.currentModel },
        )
    }

    private fun mergeMessage(
        retained: ChatGptWebMessage,
        incoming: ChatGptWebMessage,
    ): ChatGptWebMessage {
        if (retained.role != incoming.role) return incoming
        val content = if (incoming.content.length >= retained.content.length) {
            incoming.content
        } else {
            retained.content
        }
        val retainedPartsSize = retained.parts.sumOf { it.label.length }
        val incomingPartsSize = incoming.parts.sumOf { it.label.length }
        return incoming.copy(
            content = content,
            parts = if (incomingPartsSize >= retainedPartsSize) incoming.parts else retained.parts,
        )
    }

    private fun isConversationTranscript(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.messages.isNotEmpty() &&
            snapshot.pageKind == "conversation" &&
            ChatGptWebConversationPath.fromUrl(snapshot.url) != null
}

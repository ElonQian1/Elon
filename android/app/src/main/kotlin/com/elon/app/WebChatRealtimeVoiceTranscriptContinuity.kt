package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebNativeVoiceTranscriptEvent
import com.elon.app.chatgptweb.ChatGptWebNativeVoiceTranscriptUpdate
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import java.util.LinkedHashMap
import java.util.LinkedHashSet

internal class WebChatRealtimeVoiceTranscriptContinuity {
    private enum class Phase {
        IDLE,
        ACTIVE,
        RECOVERING,
    }

    private var phase = Phase.IDLE
    private var conversationPath: String? = null
    private var retainedSnapshot: ChatGptWebSnapshot? = null
    private var baselineMessageIds = emptySet<String>()
    private val liveMessages = LinkedHashMap<String, LiveMessage>()
    private val seenEventIds = LinkedHashSet<String>()

    fun begin(current: ChatGptWebSnapshot?) {
        phase = Phase.ACTIVE
        conversationPath = ChatGptWebConversationPath.fromUrl(current?.url)
        retainedSnapshot = current?.takeIf(::isConversationTranscript)
        baselineMessageIds = retainedSnapshot?.messages?.mapTo(mutableSetOf()) { it.id }.orEmpty()
        liveMessages.clear()
        seenEventIds.clear()
    }

    fun end(current: ChatGptWebSnapshot?): ChatGptWebSnapshot? {
        retain(current)
        phase = Phase.RECOVERING
        return presentationSnapshot()
    }

    fun applyLive(event: ChatGptWebNativeVoiceTranscriptEvent): ChatGptWebSnapshot? {
        if (phase != Phase.ACTIVE || retainedSnapshot == null) return null
        if (event.eventId != null && !rememberEvent(event.eventId)) return presentationSnapshot()
        val previous = liveMessages[event.streamKey]
        val nextText = when (event.update) {
            ChatGptWebNativeVoiceTranscriptUpdate.DELTA ->
                (previous?.text.orEmpty() + event.text).take(MAX_LIVE_TRANSCRIPT_CHARS)
            ChatGptWebNativeVoiceTranscriptUpdate.FINAL ->
                event.text.takeIf(String::isNotBlank) ?: previous?.text.orEmpty()
        }
        if (nextText.isBlank()) return presentationSnapshot()
        liveMessages[event.streamKey] = LiveMessage(
            id = previous?.id ?: liveMessageId(event),
            role = event.speaker.role,
            text = nextText,
            final = event.update == ChatGptWebNativeVoiceTranscriptUpdate.FINAL,
        )
        return presentationSnapshot()
    }

    fun resolve(incoming: ChatGptWebSnapshot): ChatGptWebSnapshot? = when (phase) {
        Phase.IDLE -> incoming
        Phase.ACTIVE -> {
            retain(incoming)
            presentationSnapshot()
        }
        Phase.RECOVERING -> resolveRecovery(incoming)
    }

    fun reset() {
        phase = Phase.IDLE
        conversationPath = null
        retainedSnapshot = null
        baselineMessageIds = emptySet()
        liveMessages.clear()
        seenEventIds.clear()
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
        reconcileLiveMessages(incoming)
        retain(incoming)
        return presentationSnapshot()
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

    private fun presentationSnapshot(): ChatGptWebSnapshot? {
        val retained = retainedSnapshot ?: return null
        if (liveMessages.isEmpty()) return retained
        val messages = retained.messages + liveMessages.values.map { live ->
            ChatGptWebMessage(
                id = live.id,
                role = live.role,
                content = live.text,
                state = if (live.final) "completed" else "streaming",
                parts = emptyList(),
            )
        }
        return retained.copy(
            messages = messages,
            streaming = liveMessages.values.any { !it.final },
            observedMessageCount = maxOf(retained.observedMessageCount, messages.size),
        )
    }

    private fun reconcileLiveMessages(incoming: ChatGptWebSnapshot) {
        if (liveMessages.isEmpty()) return
        val authoritative = incoming.messages.filterNot { it.id in baselineMessageIds }
        liveMessages.entries.removeAll { (_, live) ->
            authoritative.any { message ->
                message.role == live.role && authoritativeCoversLive(message.content, live.text)
            }
        }
    }

    private fun authoritativeCoversLive(authoritative: String, live: String): Boolean {
        val settled = authoritative.trim()
        val preview = live.trim()
        if (settled.isEmpty() || preview.isEmpty()) return false
        return settled == preview || settled.contains(preview)
    }

    private fun rememberEvent(eventId: String): Boolean {
        if (!seenEventIds.add(eventId)) return false
        while (seenEventIds.size > MAX_SEEN_EVENTS) {
            val oldest = seenEventIds.iterator()
            oldest.next()
            oldest.remove()
        }
        return true
    }

    private fun liveMessageId(event: ChatGptWebNativeVoiceTranscriptEvent): String =
        "elon-native-voice-${event.speaker.role}-${Integer.toHexString(event.streamKey.hashCode())}"

    private data class LiveMessage(
        val id: String,
        val role: String,
        val text: String,
        val final: Boolean,
    )

    private companion object {
        const val MAX_LIVE_TRANSCRIPT_CHARS = 64 * 1024
        const val MAX_SEEN_EVENTS = 256
    }
}

package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
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
        retainedSnapshot = when {
            retained == null -> candidate
            candidate.messageWindowStart + candidate.messages.size >=
                retained.messageWindowStart + retained.messages.size -> candidate
            else -> retained.copy(
                authenticated = candidate.authenticated,
                composerReady = candidate.composerReady,
                streaming = candidate.streaming,
                currentModel = candidate.currentModel.ifBlank { retained.currentModel },
                capabilities = candidate.capabilities,
                loginRequired = candidate.loginRequired,
            )
        }
    }

    private fun isConversationTranscript(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.messages.isNotEmpty() &&
            snapshot.pageKind == "conversation" &&
            ChatGptWebConversationPath.fromUrl(snapshot.url) != null
}

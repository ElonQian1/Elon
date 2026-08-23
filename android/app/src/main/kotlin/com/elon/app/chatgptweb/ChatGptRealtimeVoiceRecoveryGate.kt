package com.elon.app.chatgptweb

internal class ChatGptRealtimeVoiceRecoveryGate {
    internal data class Token(
        val generation: Long,
        val snapshotRevision: Long,
        val reloadAllowed: Boolean,
    )

    private var generation = 0L

    fun invalidate() {
        generation += 1
    }

    fun arm(snapshotRevision: Long, reloadAllowed: Boolean): Token {
        generation += 1
        return Token(generation, snapshotRevision, reloadAllowed)
    }

    fun shouldReload(token: Token, conversationRecoveredSince: Boolean): Boolean =
        token.generation == generation && token.reloadAllowed && !conversationRecoveredSince

    fun isCurrent(token: Token): Boolean = token.generation == generation
}

internal class ChatGptRealtimeVoiceConversationRecovery(initialSnapshot: ChatGptWebSnapshot?) {
    private var revision = if (initialSnapshot == null) 0L else 1L
    private var conversationReady = initialSnapshot.isConversationReady()

    fun revision(): Long = revision

    fun accept(snapshot: ChatGptWebSnapshot) {
        revision += 1
        conversationReady = snapshot.isConversationReady()
    }

    fun recoveredSince(baselineRevision: Long): Boolean =
        revision > baselineRevision && conversationReady

    private fun ChatGptWebSnapshot?.isConversationReady(): Boolean =
        this?.pageKind == "conversation" && composerReady
}

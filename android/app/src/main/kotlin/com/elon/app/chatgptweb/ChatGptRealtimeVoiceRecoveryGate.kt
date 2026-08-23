package com.elon.app.chatgptweb

internal class ChatGptRealtimeVoiceRecoveryGate {
    internal data class Token(
        val generation: Long,
        val snapshotRevision: Long,
    )

    private var generation = 0L

    fun invalidate() {
        generation += 1
    }

    fun arm(snapshotRevision: Long): Token {
        generation += 1
        return Token(generation, snapshotRevision)
    }

    fun shouldReload(token: Token, conversationRecoveredSince: Boolean): Boolean =
        token.generation == generation && !conversationRecoveredSince
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

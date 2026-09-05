package com.elon.app.chatgptweb

internal object ChatGptWebContentSnapshotPolicy {
    fun reconcile(previous: ChatGptWebSnapshot?, incoming: ChatGptWebSnapshot): ChatGptWebSnapshot {
        if (!incoming.contentOnly || previous == null) return incoming
        val previousId = ChatGptWebConversationPath.fromUrl(previous.url)
            ?.let(ChatGptWebConversationPath::identity) ?: return incoming
        val incomingId = ChatGptWebConversationPath.fromUrl(incoming.url)
            ?.let(ChatGptWebConversationPath::identity) ?: return incoming
        if (previousId != incomingId) return incoming
        // A history GET is not evidence about the live composer, voice, or an in-flight answer.
        if (previous.streaming) return previous
        return previous.copy(
            title = incoming.title.ifBlank { previous.title },
            url = incoming.url,
            messages = incoming.messages,
            messageWindowStart = incoming.messageWindowStart,
            observedMessageCount = incoming.observedMessageCount,
        )
    }
}

package com.elon.app.chatgptweb

internal object ChatGptWebHandshakeCompletionPolicy {
    fun completes(event: ChatGptWebEvent): Boolean = event is ChatGptWebEvent.Snapshot
}

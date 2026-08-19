package com.elon.app.chatgptweb

internal object ChatGptWebConnectionMessagePolicy {
    fun shouldShow(
        state: ChatGptBackgroundSession.State,
        hasMessages: Boolean,
        conversationNavigationPending: Boolean,
    ): Boolean = state == ChatGptBackgroundSession.State.LOADING &&
        !hasMessages &&
        !conversationNavigationPending
}

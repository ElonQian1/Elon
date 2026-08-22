package com.elon.app.chatgptweb

internal object ChatGptWebSnapshotPresentation {
    fun loadingConversation(
        cached: ChatGptWebSnapshot?,
        previous: ChatGptWebSnapshot?,
        path: String,
    ): ChatGptWebSnapshot = passive(
        base = cached,
        modelFallback = previous?.currentModel.orEmpty(),
        url = "$CHATGPT_ORIGIN$path",
        pageKind = "conversation",
    )

    fun newConversation(previous: ChatGptWebSnapshot?): ChatGptWebSnapshot = passive(
        base = null,
        modelFallback = previous?.currentModel.orEmpty(),
        url = ChatGptWebNavigationPolicy.START_URL,
        pageKind = "home",
    )

    fun revalidating(current: ChatGptWebSnapshot): ChatGptWebSnapshot = passive(
        base = current,
        modelFallback = current.currentModel,
        url = current.url,
        pageKind = current.pageKind,
    )

    private fun passive(
        base: ChatGptWebSnapshot?,
        modelFallback: String,
        url: String,
        pageKind: String,
    ) = ChatGptWebSnapshot(
        title = base?.title.orEmpty(),
        url = url,
        draft = "",
        messages = base?.messages.orEmpty(),
        authenticated = false,
        composerReady = false,
        streaming = false,
        currentModel = base?.currentModel?.ifBlank { modelFallback } ?: modelFallback,
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = pageKind,
        loginRequired = false,
        messageWindowStart = base?.messageWindowStart ?: 0,
        observedMessageCount = base?.observedMessageCount ?: 0,
    )

    private const val CHATGPT_ORIGIN = "https://chatgpt.com"
}

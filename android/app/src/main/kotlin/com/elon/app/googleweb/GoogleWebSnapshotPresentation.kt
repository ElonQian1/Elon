package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal object GoogleWebSnapshotPresentation {
    fun opening(
        cached: ChatGptWebSnapshot?,
        previous: ChatGptWebSnapshot?,
        url: String,
    ): ChatGptWebSnapshot = cached?.takeIf {
        GoogleWebNavigationPolicy.sanitizeRestorableUrl(it.url) == url
    } ?: loading(previous, url)

    fun loading(previous: ChatGptWebSnapshot?, url: String): ChatGptWebSnapshot =
        (previous ?: empty(url)).copy(
            title = "",
            url = url,
            draft = "",
            messages = emptyList(),
            authenticated = false,
            composerReady = false,
            streaming = false,
            attachments = emptyList(),
            dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
            pageKind = "conversation",
            loginRequired = false,
            messageWindowStart = 0,
            observedMessageCount = 0,
        )

    private fun empty(url: String) = ChatGptWebSnapshot(
        title = "",
        url = url,
        draft = "",
        messages = emptyList(),
        authenticated = false,
        composerReady = false,
        streaming = false,
        currentModel = "Google AI 模式",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "conversation",
    )
}

package com.elon.app.chatgptweb

import com.elon.app.WebChatBackgroundInteractionLease

internal class ChatGptComposerOptionInteraction(
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val isInteractiveSurface: () -> Boolean,
    private val backgroundLease: WebChatBackgroundInteractionLease,
) {
    fun dismiss(requestId: String?) {
        backgroundLease.release()
        pageAdapter()?.dismissComposerMenu(requestId)
    }

    fun dispatch(section: String, requestId: String?) {
        val action: () -> Unit = {
            if (section == "model") pageAdapter()?.listModelOptions(requestId)
            else pageAdapter()?.listComposerTools(requestId)
            Unit
        }
        if (isInteractiveSurface() || !backgroundLease.run(action)) action()
    }

    fun release() = backgroundLease.release()
}

internal fun chatGptComposerListAction(section: String): String =
    if (section == "model") "list_model_options" else "list_composer_tools"

internal fun chatGptComposerSectionForAction(action: String): String? = when (action) {
    "list_model_options", "collect_model_options" -> "model"
    "list_composer_tools", "collect_composer_tools" -> "tools"
    else -> null
}

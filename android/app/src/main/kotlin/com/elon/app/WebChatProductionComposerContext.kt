package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal object WebChatProductionComposerContext {
    fun projectTitle(
        index: ChatGptWebConversationIndexState,
        currentConversationPath: String?,
    ): String? {
        val currentIdentity = ChatGptWebConversationPath.identity(currentConversationPath)
        val conversation = index.conversations.firstOrNull {
            currentIdentity != null && ChatGptWebConversationPath.identity(it.path) == currentIdentity
        } ?: index.conversations.firstOrNull { it.active }
        conversation?.projectTitle.cleanLabel()?.let { return it }

        val projectId = conversation?.projectId
            ?: ChatGptWebConversationPath.projectId(currentConversationPath)
        index.projects.firstOrNull { it.id == projectId }
            ?.title
            .cleanLabel()
            ?.let { return it }
        if (currentIdentity != null && conversation != null) return null
        return index.projects.firstOrNull { it.active }?.title.cleanLabel()
    }

    fun inputHint(baseHint: String, projectTitle: String?): String {
        val project = projectTitle.cleanLabel() ?: return baseHint
        return if (baseHint == "输入内容") "${project}中的新聊天" else baseHint
    }

    private fun String?.cleanLabel(): String? = this
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

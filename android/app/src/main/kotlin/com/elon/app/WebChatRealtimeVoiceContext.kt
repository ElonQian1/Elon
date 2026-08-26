package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal data class WebChatRealtimeVoiceContext(
    val conversationPath: String?,
    val label: String,
    val savedToHistory: Boolean,
    val openable: Boolean = conversationPath != null && savedToHistory,
)

internal object WebChatRealtimeVoiceContextPolicy {
    fun resolve(
        conversationPath: String?,
        conversations: List<ChatGptWebConversation>,
        temporaryChat: Boolean,
    ): WebChatRealtimeVoiceContext {
        if (temporaryChat) {
            return WebChatRealtimeVoiceContext(
                conversationPath = null,
                label = "临时聊天（不保存到历史）",
                savedToHistory = false,
            )
        }

        val normalizedPath = ChatGptWebConversationPath.normalize(conversationPath)
        val identity = ChatGptWebConversationPath.identity(normalizedPath)
        val conversation = identity?.let { expected ->
            conversations.firstOrNull { value ->
                ChatGptWebConversationPath.identity(value.path) == expected
            }
        }
        val title = conversation?.title
            ?.trim()
            ?.takeIf { it.isNotBlank() && it !in PLACEHOLDER_TITLES }
        val project = conversation?.projectTitle?.trim()?.takeIf(String::isNotBlank)
        val label = when {
            title != null && project != null -> "$project / $title"
            title != null -> title
            normalizedPath != null -> "当前 ChatGPT 会话"
            else -> "新会话（发送后自动归档）"
        }
        return WebChatRealtimeVoiceContext(
            conversationPath = normalizedPath,
            label = label,
            savedToHistory = true,
        )
    }

    private val PLACEHOLDER_TITLES = setOf("ChatGPT", "New chat", "新聊天", "新会话")
}

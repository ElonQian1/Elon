package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal data class WebChatRealtimeVoiceContext(
    val conversationPath: String?,
    val label: String,
    val savedToHistory: Boolean,
    val openable: Boolean = conversationPath != null && savedToHistory,
)

internal class WebChatRealtimeVoiceContextTracker(
    private val resolve: () -> WebChatRealtimeVoiceContext,
    private val schedule: (Runnable, Long) -> Unit,
    private val isCurrent: (Int) -> Boolean,
    private val onChanged: (WebChatRealtimeVoiceContext) -> Unit,
) {
    var value: WebChatRealtimeVoiceContext? = null
        private set

    fun begin(): WebChatRealtimeVoiceContext = resolve().also { value = it }

    fun reset() {
        value = null
    }

    fun refresh(): Boolean {
        val next = resolve()
        if (next == value) return false
        value = next
        onChanged(next)
        return true
    }

    fun scheduleRefresh(generation: Int, attempt: Int = 0) {
        if (!isCurrent(generation) || !needsRefresh()) return
        schedule(Runnable {
            if (!isCurrent(generation)) return@Runnable
            refresh()
            if (attempt < MAX_POLLS && needsRefresh()) scheduleRefresh(generation, attempt + 1)
        }, REFRESH_DELAY_MS)
    }

    private fun needsRefresh(): Boolean = value?.let { context ->
        context.savedToHistory && (
            context.conversationPath == null || context.label == CURRENT_CONVERSATION_LABEL
        )
    } == true

    private companion object {
        const val REFRESH_DELAY_MS = 1_000L
        const val MAX_POLLS = 90
        const val CURRENT_CONVERSATION_LABEL = "当前 ChatGPT 会话"
    }
}

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

package com.elon.app.chatgptweb

import java.util.Locale

internal object ChatGptConversationFilter {
    fun apply(
        conversations: List<ChatGptWebConversation>,
        query: String,
    ): List<ChatGptWebConversation> {
        val needle = query.trim().lowercase(Locale.ROOT)
        if (needle.isEmpty()) return conversations
        return conversations.filter { conversation ->
            conversation.title.lowercase(Locale.ROOT).contains(needle)
        }
    }
}

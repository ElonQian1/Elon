package com.elon.app.chatgptweb

internal class ChatGptDeletedConversations {
    private val ids = linkedSetOf<String>()

    fun remember(values: Set<String>) {
        ids += values.filter { ChatGptWebConversationPath.identity("/c/$it") == it }
        while (ids.size > 200) ids.remove(ids.first())
    }

    fun containsPath(path: String?): Boolean = ChatGptWebConversationPath.identity(path) in ids
    fun containsUrl(url: String?): Boolean = containsPath(ChatGptWebConversationPath.fromUrl(url))
    fun ids(): Set<String> = ids.toSet()
    fun clear() = ids.clear()
}

package com.elon.app.chatgptweb

import java.util.UUID

internal class ChatGptConversationRefreshOwner {
    private var requestId: String? = null
    private var projectId: String? = null

    fun begin(scope: String?): String {
        projectId = scope
        return (PREFIX + UUID.randomUUID().toString().replace("-", "").take(29)).also { requestId = it }
    }

    fun matches(id: String?): Boolean = id != null && id == requestId

    fun matches(event: ChatGptWebEvent.ConversationList): Boolean =
        matches(event.requestId) && event.scopeProjectId == projectId

    fun isStaleNative(id: String?): Boolean = isNative(id) && !matches(id)

    fun clear() {
        requestId = null
        projectId = null
    }

    companion object {
        private const val PREFIX = "mcp_dir"
        fun isNative(id: String?): Boolean = id?.startsWith(PREFIX) == true
    }
}

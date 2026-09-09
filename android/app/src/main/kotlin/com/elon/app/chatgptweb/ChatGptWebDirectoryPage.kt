package com.elon.app.chatgptweb

import org.json.JSONObject

internal data class ChatGptWebDirectoryPage(
    val requestId: String,
    val scope: String,
    val requestedHandle: String,
    val handle: String,
    val nextHandle: String?,
    val complete: Boolean,
    val cached: Boolean,
    val conversations: List<ChatGptWebConversation>,
    val projects: List<ChatGptWebProject>,
) {
    companion object {
        const val ACTION = "browse_directory_page"
        val HANDLE = Regex("dp_[a-f0-9]{32}")
        fun validScope(value: String) = value in setOf("conversations", "projects") ||
            Regex("g-p-[A-Za-z0-9_-]{1,160}").matches(value)

        fun parse(
            event: JSONObject,
            parseConversations: (JSONObject) -> List<ChatGptWebConversation>,
            parseProjects: (JSONObject) -> List<ChatGptWebProject>,
        ): ChatGptWebDirectoryPage? {
            if (event.optInt("version") != 1) return null
            val request = event.optString("requestId").takeIf(Regex("mcp_[a-z0-9]{1,32}")::matches) ?: return null
            val scope = event.optString("scope").takeIf(::validScope) ?: return null
            val requested = event.optString("requestedHandle")
            val handle = event.optString("handle").takeIf(HANDLE::matches) ?: return null
            if (requested.isNotEmpty() && (requested != handle || !HANDLE.matches(requested))) return null
            val next = if (event.isNull("nextHandle")) null else event.optString("nextHandle").takeIf(HANDLE::matches) ?: return null
            val complete = event.opt("complete") as? Boolean ?: return null
            val cached = event.opt("cached") as? Boolean ?: return null
            if (next == handle || complete && next != null) return null
            val rawChats = event.optJSONArray("conversations") ?: return null
            val rawProjects = event.optJSONArray("projects") ?: return null
            if (rawChats.length() > 200 || rawProjects.length() > 40 ||
                scope == "projects" && rawChats.length() != 0 || scope != "projects" && rawProjects.length() != 0) return null
            val chats = parseConversations(event)
            val projects = parseProjects(event)
            if (chats.size != rawChats.length() || projects.size != rawProjects.length()) return null
            if (chats.any { row -> ChatGptWebConversationPath.identity(row.path) != row.id ||
                    scope.startsWith("g-p-") && row.projectId != scope } ||
                projects.any { ChatGptWebConversationPath.projectId(it.path) != it.id }) return null
            return ChatGptWebDirectoryPage(request, scope, requested, handle, next, complete, cached, chats, projects)
        }
    }
}

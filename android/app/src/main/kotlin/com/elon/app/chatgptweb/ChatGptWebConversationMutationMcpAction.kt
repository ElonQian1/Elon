package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebConversationMutationMcpAction {
    fun dispatch(
        args: JSONObject,
        commands: ChatGptWebMcpCommandPort,
        snapshot: ChatGptWebSnapshot? = null,
        conversations: List<ChatGptWebConversation> = emptyList(),
        dispatchCommand: (String, (String) -> Unit) -> Unit,
    ): String? {
        val path = ChatGptWebConversationPath.normalize(args.optString("conversation_path"))
        if (args.optString("action") == "chatgpt_share_conversation" && args.has("operation")) {
            val resource = if (args.has("resource")) args.opt("resource") as? String else "conversation"
            if (resource !in setOf("conversation", "canvas")) return "share_invalid_selection"
            val request = (if (args.optString("operation") == "list_account") {
                if (args.has("conversation_path") || args.has("share_id")) return "share_invalid_selection"
                val offset = if (args.has("page_offset")) args.opt("page_offset") as? Int
                    ?: return "share_invalid_selection" else 0
                val selection = if (args.has("selection_ticket")) args.opt("selection_ticket") as? String
                    ?: return "share_invalid_selection" else null
                ChatGptWebSharedLinks.accountRequest(offset, selection, canvas = resource == "canvas")
            } else if (args.optString("operation") in setOf("read_account", "revoke_account") && resource == "canvas") {
                if (args.has("conversation_path")) return "share_invalid_selection"
                val id = args.opt("share_id") as? String ?: return "share_invalid_selection"
                val ticket = args.opt("selection_ticket") as? String ?: return "share_invalid_selection"
                if (args.optString("operation") == "read_account") ChatGptWebCanvasContent.request(id, ticket)
                else ChatGptWebSharedLinks.canvasRevokeRequest(id, ticket)
            } else {
                if (resource != "conversation") return "share_invalid_selection"
                if (path == null) return "invalid_conversation_path"
                ChatGptWebSharedLinks.request(path, args.optString("operation"),
                    args.optString("share_id"), args.optString("selection_ticket"))
            }) ?: return "share_invalid_selection"
            if (request.getString("operation") in setOf("revoke", "revoke_account") && args.opt("user_confirmed") != true) {
                return "user_confirmation_required"
            }
            val page = runCatching { java.net.URI(snapshot?.url ?: "") }.getOrNull()
            if (page?.scheme != "https" || page.host != "chatgpt.com" || page.port != -1 || page.userInfo != null) {
                return "share_context_unavailable"
            }
            dispatchCommand("share_conversation") { requestId -> commands.manageConversationShares(request, requestId) }
            return null
        }
        if (path == null) return "invalid_conversation_path"
        if (!args.optBoolean("user_confirmed", false)) return "user_confirmation_required"
        when (args.optString("action")) {
            "chatgpt_share_conversation" -> {
                if (snapshot == null || ChatGptWebConversationPath.identity(ChatGptWebConversationPath.fromUrl(snapshot.url)) !=
                    ChatGptWebConversationPath.identity(path)) return "share_context_unavailable"
                if (snapshot.streaming) return "share_conversation_busy"
                dispatchCommand("share_conversation") { requestId -> commands.shareConversation(path, requestId) }
            }
            "chatgpt_delete_conversation" -> {
                val id = ChatGptWebConversationPath.identity(path)
                if (snapshot == null) return "delete_context_unavailable"
                if (conversations.none { ChatGptWebConversationPath.identity(it.path) == id }) return "delete_selection_expired"
                dispatchCommand("delete_conversation") { requestId -> commands.deleteConversation(path, requestId) }
            }
            "chatgpt_set_conversation_pinned" -> {
                val pinned = args.opt("pinned") as? Boolean ?: return "missing_pinned"
                dispatchCommand("set_conversation_pinned") { requestId ->
                    commands.setConversationPinned(path, pinned, requestId)
                }
            }
            "chatgpt_set_conversation_archived" -> {
                val archived = args.opt("archived") as? Boolean ?: return "missing_archived"
                dispatchCommand("set_conversation_archived") { requestId ->
                    commands.setConversationArchived(path, archived, requestId)
                }
            }
            "chatgpt_rename_conversation" -> {
                val title = args.optString("title").trim()
                if (title.isBlank() || title.length > MAX_TITLE_LENGTH) return "invalid_title"
                dispatchCommand("rename_conversation") { requestId ->
                    commands.renameConversation(path, title, requestId)
                }
            }
            "chatgpt_move_conversation_to_project" -> {
                val projectId = ChatGptWebConversationPath.canonicalProjectId(
                    args.optString("project_id"),
                ) ?: return "invalid_project_id"
                val conversationTitle = args.optString("conversation_title").trim()
                if (conversationTitle.length > MAX_TITLE_LENGTH) return "invalid_title"
                dispatchCommand("move_conversation_to_project") { requestId ->
                    commands.moveConversationToProject(
                        path,
                        conversationTitle,
                        projectId,
                        requestId,
                    )
                }
            }
            else -> return "unsupported_conversation_mutation"
        }
        return null
    }

    private const val MAX_TITLE_LENGTH = 160
}

package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebConversationMutationMcpAction {
    fun dispatch(
        args: JSONObject,
        commands: ChatGptWebMcpCommandPort,
        dispatchCommand: (String, (String) -> Unit) -> Unit,
    ): String? {
        val path = ChatGptWebConversationPath.normalize(args.optString("conversation_path"))
            ?: return "invalid_conversation_path"
        if (!args.optBoolean("user_confirmed", false)) return "user_confirmation_required"
        when (args.optString("action")) {
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

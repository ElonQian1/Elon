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
        val pinned = args.opt("pinned") as? Boolean ?: return "missing_pinned"
        if (!args.optBoolean("user_confirmed", false)) return "user_confirmation_required"
        dispatchCommand("set_conversation_pinned") { requestId ->
            commands.setConversationPinned(path, pinned, requestId)
        }
        return null
    }
}

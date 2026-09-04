package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebPrivateReadAloudMcpAction {
    fun dispatch(
        args: JSONObject,
        snapshot: ChatGptWebSnapshot?,
        commands: ChatGptWebMcpCommandPort,
        dispatchCommand: (String, (String) -> Unit) -> Unit,
    ): String? {
        if (snapshot?.privateReadAloudReady != true) return "private_read_aloud_not_ready"
        val contextId = args.optString("context_id").trim()
        if (!MESSAGE_CONTEXT_ID.matches(contextId)) return "invalid_context_id"
        val message = snapshot.messages.firstOrNull { it.id == contextId }
            ?: return "stale_message_id"
        if (message.role != "assistant" || message.state != "completed") {
            return "read_aloud_unavailable"
        }
        ChatGptWebPrivateResearchEventRecorder.beginVoiceWindow()
        dispatchCommand("toggle_private_read_aloud") { requestId ->
            commands.togglePrivateReadAloud(contextId, requestId)
        }
        return null
    }

    private val MESSAGE_CONTEXT_ID = Regex("[A-Za-z0-9_.:-]{1,160}")
}

package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal data class ChatGptClipboardMetadata(
    val hasPrimaryClip: Boolean,
    val itemCount: Int,
    val mimeTypes: Set<String>,
)

internal object ChatGptWebCopyAction {
    fun execute(
        snapshot: ChatGptWebSnapshot?,
        copyText: (String) -> ChatGptClipboardMetadata,
    ): JSONObject {
        val action = "chatgpt_copy_last_response"
        if (snapshot?.streaming == true) return error(action, "generation_in_progress")
        if (snapshot?.capabilities?.supports(ChatGptWebCapabilityId.MESSAGE_COPY) != true) {
            return error(action, "copy_unavailable")
        }
        val message = snapshot.messages.lastOrNull { it.role == "assistant" }
            ?.takeIf { it.state == "completed" && it.content.isNotBlank() }
            ?: return error(action, "copy_unavailable")
        val metadata = runCatching { copyText(message.content) }
            .getOrElse { return error(action, "clipboard_write_failed") }
        if (!metadata.hasPrimaryClip || metadata.itemCount < 1) {
            return error(action, "clipboard_write_failed")
        }
        return JSONObject()
            .put("control_ok", true)
            .put("action", action)
            .put("receipt", JSONObject()
                .put("schema", "elon.chatgpt_web.clipboard_receipt.v1")
                .put("copied", true)
                .put("source_role", "assistant")
                .put("item_count", metadata.itemCount)
                .put("mime_types", JSONArray(metadata.mimeTypes.sorted()))
                .put("content_exported", false)
            )
    }

    private fun error(action: String, code: String): JSONObject = JSONObject()
        .put("control_ok", false)
        .put("action", action)
        .put("error", code)
}

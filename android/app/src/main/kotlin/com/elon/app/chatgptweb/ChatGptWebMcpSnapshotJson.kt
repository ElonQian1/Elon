package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebMcpSnapshotJson {
    private const val MAX_MESSAGES = 50
    private const val MAX_MESSAGE_CHARS = 30_000
    private const val CONVERSATION_SUMMARY_SCHEMA = "elon.chatgpt_web.conversation_summary.v2"

    fun conversation(value: ChatGptWebSnapshot?): Any {
        if (value == null) return JSONObject.NULL
        val exportedMessages = value.messages.takeLast(MAX_MESSAGES)
        val exportedStart = value.messageWindowStart + value.messages.size - exportedMessages.size
        val windowEnd = value.messageWindowStart + value.messages.size
        return JSONObject()
            .put("schema", CONVERSATION_SUMMARY_SCHEMA)
            .put("title", value.title)
            .put("url", value.url)
            .put("current_model", value.currentModel)
            .put("message_count", value.observedMessageCount)
            .put("available_message_count", value.messages.size)
            .put("message_window_start", value.messageWindowStart)
            .put("message_window_end", windowEnd)
            .put("history_truncated", value.messageWindowStart > 0)
            .put("context_complete", value.messageWindowStart == 0 && windowEnd >= value.observedMessageCount)
            .put("exported_message_count", exportedMessages.size)
            .put("exported_message_offset", exportedStart)
            .put("messages_truncated",
                exportedMessages.size < value.messages.size || windowEnd < value.observedMessageCount)
            .put("context_action", "chatgpt_get_context")
            .put("messages", ChatGptWebMessageJson.encode(
                exportedMessages, exportedStart, MAX_MESSAGE_CHARS))
            .put("attachments", JSONArray().apply {
                value.attachments.forEach { attachment ->
                    put(JSONObject().put("id", attachment.id).put("name", attachment.name).put("state", attachment.state))
                }
            })
    }

    fun navigation(value: ChatGptWebObservedState.Snapshot): JSONObject = JSONObject()
        .put("conversation_count", value.conversations.size)
        .put("conversation_collection", ChatGptWebConversationCollectionJson.encode(value.conversationCollection))
        .put("feature_count", value.features.size)
        .put("composer_sections", JSONArray(value.composerSections.keys.sorted()))
        .put("cached_at_ms", value.updatedAtMs)

    fun conversationFiles(value: ChatGptWebObservedState.Snapshot, url: String?, nowMs: Long = System.currentTimeMillis()): Any {
        if (!value.adapterCurrent) return JSONObject.NULL
        val path = ChatGptWebConversationPath.fromUrl(url) ?: return JSONObject.NULL
        val index = value.conversationFiles[ChatGptWebConversationPath.identity(path)] ?: return JSONObject.NULL
        val receipt = value.recentCommandResults[ChatGptWebConversationFiles.ACTION]?.result
        if (index.path != path || receipt?.ok != true || receipt.requestId != index.requestId) return JSONObject.NULL
        val fresh = index.isFresh(nowMs)
        return JSONObject()
            .put("conversation_path", path)
            .put("request_id", index.requestId)
            .put("cached_at_ms", index.savedAtMs)
            .put("stale", !fresh)
            .put("truncated", index.truncated || index.files.size > 100)
            .put("files", JSONArray().apply {
                index.files.take(100).forEach { file ->
                    put(JSONObject()
                        .put("file_id", file.id).put("message_id", file.messageId)
                        .put("name", file.name).put("kind", file.kind)
                        .put("role", file.role).put("media_type", file.mediaType)
                        .put("download_handle", file.downloadHandle.takeIf {
                            fresh && ChatGptWebFileDownloadPolicy.HANDLE.matches(it)
                        }.orEmpty()))
                }
            })
    }
}

package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebMessageJson {
    fun encode(messages: List<ChatGptWebMessage>, startIndex: Int, maxChars: Int): JSONArray =
        JSONArray().apply {
            messages.forEachIndexed { offset, message ->
                put(messageJson(message, startIndex + offset, maxChars))
            }
        }

    private fun messageJson(message: ChatGptWebMessage, index: Int, maxChars: Int): JSONObject =
        JSONObject()
            .put("index", index)
            .put("id", message.id)
            .put("role", message.role)
            .put("state", message.state)
            .put("content", message.content.take(maxChars))
            .put("content_chars", message.content.length)
            .put("content_truncated", message.content.length > maxChars)
            .put("part_count", message.parts.size)
            .put("native_action", "chatgpt_reveal_message")
            .put("native_reveal_targets", JSONArray(ChatGptNativeMessageRevealTarget.ALL))
            .put(
                "native_adb_content_description",
                ChatGptNativeControlPresentation.messageSelector(message.id, message.role),
            )
            .put("parts_truncated", message.parts.size > MAX_PARTS)
            .put("parts", partsJson(message))

    private fun partsJson(message: ChatGptWebMessage): JSONArray = JSONArray().apply {
        message.parts.take(MAX_PARTS).forEachIndexed { index, part ->
            put(JSONObject()
                .put("type", part.type)
                .put("label", part.label.take(MAX_LABEL_CHARS))
                .put("metadata", metadataJson(part.metadata))
                .put(
                    "native_adb_content_description",
                    ChatGptNativeControlPresentation.messagePartSelector(
                        message.id,
                        index,
                        part.type,
                    ),
                )
                .put("label_truncated", part.label.length > MAX_LABEL_CHARS)
            )
        }
    }

    private fun metadataJson(value: ChatGptWebMessagePartMetadata?): Any {
        if (value == null) return JSONObject.NULL
        return JSONObject()
            .put("schema", METADATA_SCHEMA)
            .apply {
                value.kind?.let { put("kind", it) }
                value.language?.let { put("language", it) }
                value.mediaType?.let { put("media_type", it) }
                value.targetKind?.let { put("target_kind", it) }
                value.targetHost?.let { put("target_host", it) }
                value.lineCount?.let { put("line_count", it) }
                value.rowCount?.let { put("row_count", it) }
                value.columnCount?.let { put("column_count", it) }
            }
    }

    internal const val METADATA_SCHEMA = "elon.chatgpt_web.message_part_metadata.v1"
    private const val MAX_PARTS = 16
    private const val MAX_LABEL_CHARS = 180
}

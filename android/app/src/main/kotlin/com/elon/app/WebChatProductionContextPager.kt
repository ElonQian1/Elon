package com.elon.app

import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import org.json.JSONArray
import org.json.JSONObject

/** Provider-neutral context paging for the real friend-chat surface. */
internal object WebChatProductionContextPager {
    const val SCHEMA = "elon.web_chat.production_context.v1"

    fun page(
        providerId: WebChatProviderId,
        conversationPath: String?,
        model: String,
        state: String,
        streaming: Boolean,
        messages: List<ChatMessage>,
        args: JSONObject,
    ): JSONObject {
        val revision = revision(providerId, conversationPath, messages)
        val cursorText = args.optString("message_cursor").trim()
        val cursor = if (cursorText.isBlank()) null else parseCursor(cursorText)
            ?: return error("invalid_message_cursor", revision, messages.size)
        if (cursor != null && cursor.revision != revision) {
            return error("stale_message_cursor", revision, messages.size)
                .put("retry_from_message_offset", 0)
        }
        val offset = (cursor?.offset ?: args.optInt("message_offset", 0))
            .coerceIn(0, messages.size)
        val limit = args.optInt("message_limit", DEFAULT_LIMIT).coerceIn(1, MAX_LIMIT)
        val end = (offset + limit).coerceAtMost(messages.size)
        val nextOffset = end.takeIf { it < messages.size }
        return JSONObject()
            .put("control_ok", true)
            .put("action", "get_web_chat_context")
            .put("schema", SCHEMA)
            .put("provider_id", providerId.wireValue)
            .put("conversation_path", conversationPath ?: JSONObject.NULL)
            .put("model", model.take(MAX_MODEL_CHARS))
            .put("state", state)
            .put("streaming", streaming)
            .put("cursor_stable", !streaming)
            .put("context_revision", revision)
            .put("message_count", messages.size)
            .put("message_offset", offset)
            .put("message_limit", limit)
            .put("message_cursor", cursor(revision, offset))
            .put("next_message_offset", nextOffset ?: JSONObject.NULL)
            .put("next_message_cursor", nextOffset?.let { cursor(revision, it) } ?: JSONObject.NULL)
            .put("has_more", nextOffset != null)
            .put("messages", JSONArray().apply {
                messages.subList(offset, end).forEachIndexed { index, message ->
                    put(messageJson(offset + index, message))
                }
            })
    }

    private fun messageJson(index: Int, message: ChatMessage): JSONObject {
        val content = message.content.take(MAX_CONTENT_CHARS)
        val metadata = message.webChatMessage
        return JSONObject()
            .put("index", index)
            .put("id", message.id ?: JSONObject.NULL)
            .put("role", message.role)
            .put("content", content)
            .put("content_chars", message.content.length)
            .put("content_truncated", content.length < message.content.length)
            .put("created_at_ms", message.createdAtMs)
            .put("send_status", message.sendStatus ?: JSONObject.NULL)
            .put("model_used", message.modelUsed ?: JSONObject.NULL)
            .put("source_message_id", metadata?.sourceMessageId ?: JSONObject.NULL)
            .put("render_markdown", metadata?.renderMarkdown == true)
            .put("actions", JSONArray().apply {
                WebChatProductionMessageActionPolicy.resolve(message)
                    .sortedBy(WebChatMessageAction::wireValue)
                    .forEach { put(it.wireValue) }
            })
            .put("parts", JSONArray().apply {
                metadata?.contentParts.orEmpty().take(MAX_PARTS).forEach { part ->
                    put(JSONObject()
                        .put("type", part.type.take(MAX_LABEL_CHARS))
                        .put("label", part.label.take(MAX_LABEL_CHARS))
                        .put("language", part.language ?: JSONObject.NULL)
                        .put("media_type", part.mediaType ?: JSONObject.NULL)
                        .put("target_host", part.targetHost ?: JSONObject.NULL)
                        .put("line_count", part.lineCount ?: JSONObject.NULL)
                        .put("row_count", part.rowCount ?: JSONObject.NULL)
                        .put("column_count", part.columnCount ?: JSONObject.NULL))
                }
            })
            .put("part_count", metadata?.contentParts?.size ?: 0)
            .put("parts_truncated", metadata?.contentParts.orEmpty().size > MAX_PARTS)
            .put("attachments", JSONArray().apply {
                message.attachments.orEmpty().take(MAX_ATTACHMENTS).forEach { attachment ->
                    put(JSONObject()
                        .put("kind", attachment.kind ?: JSONObject.NULL)
                        .put("display_name", attachment.displayName ?: attachment.fileName ?: JSONObject.NULL)
                        .put("mime_type", attachment.mimeType ?: JSONObject.NULL)
                        .put("size_bytes", attachment.sizeBytes ?: JSONObject.NULL)
                        .put("image_width", attachment.imageWidth ?: JSONObject.NULL)
                        .put("image_height", attachment.imageHeight ?: JSONObject.NULL)
                        .put("duration_seconds", attachment.durationSeconds ?: JSONObject.NULL))
                }
            })
            .put("attachment_count", message.attachments.orEmpty().size)
            .put("attachments_truncated", message.attachments.orEmpty().size > MAX_ATTACHMENTS)
    }

    private fun revision(
        providerId: WebChatProviderId,
        conversationPath: String?,
        messages: List<ChatMessage>,
    ): String {
        val digest = MessageDigest.getInstance("SHA-256")
        fun add(value: String) {
            digest.update(value.toByteArray(StandardCharsets.UTF_8))
            digest.update(0.toByte())
        }
        add(providerId.wireValue)
        add(conversationPath.orEmpty())
        messages.forEachIndexed { index, message ->
            add(index.toString())
            add(message.id.orEmpty())
            add(message.role)
            add(message.content)
            add(message.createdAtMs.toString())
        }
        return digest.digest().joinToString("") { "%02x".format(it) }.take(REVISION_CHARS)
    }

    private fun cursor(revision: String, offset: Int): String = "ctx1.$revision.$offset"

    private fun parseCursor(value: String): Cursor? {
        val match = CURSOR.matchEntire(value) ?: return null
        return Cursor(match.groupValues[1], match.groupValues[2].toIntOrNull() ?: return null)
    }

    private fun error(code: String, revision: String, count: Int): JSONObject = JSONObject()
        .put("control_ok", false)
        .put("action", "get_web_chat_context")
        .put("schema", SCHEMA)
        .put("error", code)
        .put("current_context_revision", revision)
        .put("message_count", count)

    private data class Cursor(val revision: String, val offset: Int)

    private const val REVISION_CHARS = 24
    private val CURSOR = Regex("^ctx1\\.([0-9a-f]{$REVISION_CHARS})\\.(\\d{1,9})$")
    private const val DEFAULT_LIMIT = 20
    private const val MAX_LIMIT = 40
    private const val MAX_CONTENT_CHARS = 30_000
    private const val MAX_MODEL_CHARS = 160
    private const val MAX_LABEL_CHARS = 240
    private const val MAX_PARTS = 24
    private const val MAX_ATTACHMENTS = 16
}

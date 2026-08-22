package com.elon.app.chatgptweb

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.io.FileOutputStream
import org.json.JSONArray
import org.json.JSONObject

internal data class WebChatSnapshotCache(
    val snapshot: ChatGptWebSnapshot,
    val savedAtMs: Long,
)

internal class WebChatSnapshotStore(
    context: Context,
    providerKey: String,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val file = AtomicFile(File(context.noBackupFilesDir, fileName(providerKey)))

    fun restore(): ChatGptWebSnapshot? {
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > WebChatSnapshotCachePolicy.MAX_FILE_BYTES) return null
        val cache = WebChatSnapshotCacheCodec.decode(bytes.toString(Charsets.UTF_8)) ?: return null
        if (!WebChatSnapshotCachePolicy.isUsable(cache.savedAtMs, nowMs())) return null
        return cache.snapshot
    }

    fun save(snapshot: ChatGptWebSnapshot) {
        val payload = WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot, nowMs()),
        ).toByteArray(Charsets.UTF_8)
        if (payload.size > WebChatSnapshotCachePolicy.MAX_FILE_BYTES) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    fun clear() = file.delete()

    internal companion object {
        fun fileName(providerKey: String): String {
            require(PROVIDER_KEY.matches(providerKey)) { "Invalid web chat provider cache key" }
            return "web-chat-$providerKey-snapshot-v1.json"
        }

        private val PROVIDER_KEY = Regex("[a-z0-9_]{2,24}")
    }
}

internal object WebChatSnapshotCacheCodec {
    private const val SCHEMA = "elon.web_chat.snapshot_cache.v1"
    private const val MAX_MESSAGES = 80
    private const val MAX_MESSAGE_CHARS = 12_000
    private const val MAX_PARTS = 8
    private const val MAX_PART_LABEL = 180
    private const val MAX_ID = 160

    fun encode(cache: WebChatSnapshotCache): String {
        val source = cache.snapshot.messages.takeLast(MAX_MESSAGES)
        val dropped = cache.snapshot.messages.size - source.size
        return JSONObject()
            .put("schema", SCHEMA)
            .put("saved_at_ms", cache.savedAtMs)
            .put("title", cache.snapshot.title.take(120))
            .put("url", cache.snapshot.url.take(2_048))
            .put("current_model", cache.snapshot.currentModel.take(80))
            .put("page_kind", cache.snapshot.pageKind.take(32))
            .put("message_window_start", cache.snapshot.messageWindowStart + dropped)
            .put("observed_message_count", cache.snapshot.observedMessageCount)
            .put("messages", JSONArray().apply {
                source.forEach { message -> put(messageJson(message)) }
            })
            .toString()
    }

    fun decode(raw: String): WebChatSnapshotCache? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        val savedAtMs = root.optLong("saved_at_ms", -1L)
        if (savedAtMs < 0L) return null
        val values = root.optJSONArray("messages") ?: return null
        val messages = buildList {
            for (index in 0 until minOf(values.length(), MAX_MESSAGES)) {
                val value = values.optJSONObject(index) ?: continue
                val role = value.optString("role").takeIf(SUPPORTED_ROLES::contains) ?: continue
                val content = value.optString("content").take(MAX_MESSAGE_CHARS)
                val parts = parts(value.optJSONArray("parts"))
                if (content.isBlank() && parts.isEmpty()) continue
                add(ChatGptWebMessage(
                    id = value.optString("id").take(MAX_ID).ifBlank { "$role-$index" },
                    role = role,
                    content = content,
                    state = "completed",
                    parts = parts,
                ))
            }
        }
        val windowStart = root.optInt("message_window_start", 0).coerceIn(0, MAX_OBSERVED_MESSAGES)
        val observedCount = root.optInt("observed_message_count", windowStart + messages.size)
            .coerceIn(windowStart + messages.size, MAX_OBSERVED_MESSAGES)
        return WebChatSnapshotCache(
            snapshot = ChatGptWebSnapshot(
                title = root.optString("title").trim().take(120),
                url = root.optString("url").take(2_048),
                draft = "",
                messages = messages,
                authenticated = false,
                composerReady = false,
                streaming = false,
                currentModel = root.optString("current_model").trim().take(80),
                attachments = emptyList(),
                dictationActive = false,
                capabilities = ChatGptWebCapabilities.EMPTY,
                pageKind = root.optString("page_kind").trim().take(32).ifBlank { "unknown" },
                loginRequired = false,
                messageWindowStart = windowStart,
                observedMessageCount = observedCount,
            ),
            savedAtMs = savedAtMs,
        )
    }

    private fun messageJson(message: ChatGptWebMessage) = JSONObject()
        .put("id", message.id.take(MAX_ID))
        .put("role", message.role)
        .put("content", message.content.take(MAX_MESSAGE_CHARS))
        .put("parts", JSONArray().apply {
            message.parts.take(MAX_PARTS).forEach { part ->
                put(JSONObject()
                    .put("type", part.type)
                    .put("label", part.label.take(MAX_PART_LABEL))
                    .put("metadata", metadataJson(part.metadata)))
            }
        })

    private fun parts(values: JSONArray?): List<ChatGptWebMessagePart> = buildList {
        if (values == null) return@buildList
        for (index in 0 until minOf(values.length(), MAX_PARTS)) {
            val value = values.optJSONObject(index) ?: continue
            val type = value.optString("type").takeIf(SUPPORTED_PARTS::contains) ?: continue
            val label = value.optString("label").trim().take(MAX_PART_LABEL)
            if (label.isBlank()) continue
            add(ChatGptWebMessagePart(type, label, metadata(value.optJSONObject("metadata"))))
        }
    }

    private fun metadataJson(value: ChatGptWebMessagePartMetadata?): Any = value?.let {
        JSONObject()
            .put("kind", it.kind)
            .put("language", it.language)
            .put("media_type", it.mediaType)
            .put("target_kind", it.targetKind)
            .put("target_host", it.targetHost)
            .put("line_count", it.lineCount)
            .put("row_count", it.rowCount)
            .put("column_count", it.columnCount)
    } ?: JSONObject.NULL

    private fun metadata(value: JSONObject?): ChatGptWebMessagePartMetadata? {
        if (value == null) return null
        return ChatGptWebMessagePartMetadata(
            kind = value.stringOrNull("kind", 32),
            language = value.stringOrNull("language", 32),
            mediaType = value.stringOrNull("media_type", 128),
            targetKind = value.stringOrNull("target_kind", 32),
            targetHost = value.stringOrNull("target_host", 253),
            lineCount = value.positiveIntOrNull("line_count"),
            rowCount = value.positiveIntOrNull("row_count"),
            columnCount = value.positiveIntOrNull("column_count"),
        ).takeUnless { it.isEmpty }
    }

    private fun JSONObject.stringOrNull(key: String, maxLength: Int): String? =
        optString(key).trim().take(maxLength).takeIf(String::isNotBlank)

    private fun JSONObject.positiveIntOrNull(key: String): Int? =
        optInt(key, 0).takeIf { it > 0 }

    private val SUPPORTED_ROLES = setOf("user", "assistant")
    private const val MAX_OBSERVED_MESSAGES = 10_000
    private val SUPPORTED_PARTS = setOf(
        "image", "file", "citation", "code", "table", "artifact", "audio", "video",
        "math", "chart", "map", "interactive",
    )
}

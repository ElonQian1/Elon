package com.elon.app.chatgptweb

import android.content.Context
import android.util.AtomicFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

internal data class ChatGptConversationHistoryCache(
    val conversations: List<ChatGptWebConversation>,
    val savedAtMs: Long,
)

internal class ChatGptConversationHistoryStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))

    fun restore(): ChatGptConversationHistoryCache? {
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > MAX_BYTES) return null
        val cache = ChatGptConversationHistoryCodec.decode(bytes.toString(Charsets.UTF_8))
            ?: return null
        if (nowMs() - cache.savedAtMs !in 0..MAX_AGE_MS) return null
        return cache
    }

    fun save(conversations: List<ChatGptWebConversation>) {
        if (conversations.isEmpty()) {
            clear()
            return
        }
        val payload = ChatGptConversationHistoryCodec.encode(
            ChatGptConversationHistoryCache(conversations.take(MAX_ITEMS), nowMs()),
        ).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    fun clear() {
        file.delete()
    }

    private companion object {
        const val FILE_NAME = "chatgpt-conversation-index-v1.json"
        const val MAX_ITEMS = 100
        const val MAX_BYTES = 64 * 1024
        const val MAX_AGE_MS = 7L * 24L * 60L * 60L * 1_000L
    }
}

internal object ChatGptConversationHistoryCodec {
    private const val SCHEMA = "elon.chatgpt_web.conversation_index.v1"
    private const val MAX_ITEMS = 100
    private const val MAX_ID_LENGTH = 160
    private const val MAX_TITLE_LENGTH = 160
    private val SAFE_PATH = Regex("/c/[A-Za-z0-9_-]{1,160}")

    fun encode(cache: ChatGptConversationHistoryCache): String = JSONObject()
        .put("schema", SCHEMA)
        .put("saved_at_ms", cache.savedAtMs)
        .put("conversations", JSONArray().apply {
            cache.conversations.take(MAX_ITEMS).forEach { conversation ->
                put(JSONObject()
                    .put("id", conversation.id.take(MAX_ID_LENGTH))
                    .put("title", conversation.title.take(MAX_TITLE_LENGTH))
                    .put("path", conversation.path)
                )
            }
        })
        .toString()

    fun decode(raw: String): ChatGptConversationHistoryCache? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        val savedAtMs = root.optLong("saved_at_ms", -1L)
        if (savedAtMs < 0L) return null
        val values = root.optJSONArray("conversations") ?: return null
        val conversations = buildList {
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(values.length(), MAX_ITEMS)) {
                val value = values.optJSONObject(index) ?: continue
                val path = value.optString("path")
                val title = value.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (!SAFE_PATH.matches(path) || title.isBlank() || !seen.add(path)) continue
                add(ChatGptWebConversation(
                    id = value.optString("id").ifBlank { path.removePrefix("/c/") }
                        .take(MAX_ID_LENGTH),
                    title = title,
                    path = path,
                    active = false,
                ))
            }
        }
        if (conversations.isEmpty()) return null
        return ChatGptConversationHistoryCache(conversations, savedAtMs)
    }
}

package com.elon.app.chatgptweb

import com.elon.app.WebChatConversationFile
import com.elon.app.WebChatConversationFileIndex
import org.json.JSONObject

internal object ChatGptWebConversationFiles {
    const val ACTION = "list_conversation_files"

    fun parse(event: JSONObject): WebChatConversationFileIndex? {
        val path = ChatGptWebConversationPath.normalize(event.optString("conversationPath")) ?: return null
        val requestId = event.optString("requestId").takeIf { REQUEST_ID.matches(it) } ?: return null
        val items = event.optJSONArray("files") ?: return null
        val files = buildList {
            for (index in 0 until minOf(items.length(), 100)) {
                val item = items.optJSONObject(index) ?: continue
                val id = item.optString("id").takeIf { ITEM_ID.matches(it) } ?: continue
                val messageId = item.optString("messageId").takeIf { MESSAGE_ID.matches(it) } ?: continue
                val name = item.optString("name").trim().take(180).takeIf(String::isNotEmpty) ?: continue
                val kind = item.optString("kind").takeIf { it == "image" || it == "file" } ?: continue
                val role = item.optString("role").takeIf { it == "user" || it == "assistant" } ?: continue
                add(WebChatConversationFile(id, messageId, name, kind, role,
                    item.optString("mediaType").takeIf { MIME.matches(it) }.orEmpty()))
            }
        }.distinctBy { it.id }
        // Malformed descriptors cannot be advertised as a complete empty file list.
        return WebChatConversationFileIndex(path, requestId, files,
            event.optBoolean("truncated") || items.length() > 100 || files.size != items.length())
    }

    private val REQUEST_ID = Regex("mcp_[a-z0-9]{1,32}")
    private val ITEM_ID = Regex("[A-Za-z0-9_-]{1,180}:[0-9]{1,3}")
    private val MESSAGE_ID = Regex("[A-Za-z0-9_-]{1,180}")
    private val MIME = Regex("[A-Za-z0-9.+-]{1,63}/[A-Za-z0-9.+-]{1,63}")
}

internal class ChatGptWebConversationFileCache {
    private val indexes = linkedMapOf<String, WebChatConversationFileIndex>()

    fun accept(value: WebChatConversationFileIndex, nowMs: Long) {
        val id = ChatGptWebConversationPath.identity(value.path) ?: return
        indexes.remove(id)
        indexes[id] = value.copy(savedAtMs = nowMs)
        while (indexes.size > 8) indexes.remove(indexes.keys.first())
    }

    fun snapshot(): Map<String, WebChatConversationFileIndex> = indexes.toMap()
    fun clear() = indexes.clear()
}

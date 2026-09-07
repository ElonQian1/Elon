package com.elon.app.chatgptweb

import java.time.Instant
import org.json.JSONObject

/** Conversation-scoped results only; never accepts arbitrary URLs or account headers. */
internal object ChatGptWebSharedLinks {
    private const val SCHEMA = "elon.conversation_shares.v1"
    private val uuid = Regex("[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")
    private val ticket = Regex("sl_[a-f0-9]{32}_[a-z0-9]{1,12}")
    private val time = Regex("\\d{4}-\\d{2}-\\d{2}T[0-9:.]+(?:Z|[+-]\\d{2}:\\d{2})")

    data class Link(val id: String, val createdAt: String?) {
        val url: String get() = "https://chatgpt.com/share/$id"
    }
    data class Index(val path: String, val ticket: String, val complete: Boolean, val items: List<Link>)

    fun path(raw: String): String? = ChatGptWebConversationPath.normalize(raw)
        ?.let(ChatGptWebConversationPath::identity)?.takeIf(uuid::matches)?.let { "/c/$it" }

    fun request(path: String, operation: String, id: String?, selection: String?): JSONObject? {
        val canonical = path(path) ?: return null
        if (operation !in setOf("list", "revoke")) return null
        if (operation == "revoke" && (id == null || !uuid.matches(id) || selection == null || !ticket.matches(selection))) return null
        return JSONObject().put("operation", operation).put("path", canonical).apply {
            if (operation == "revoke") put("id", id).put("ticket", selection)
        }
    }

    fun parse(raw: String?): Index? = runCatching {
        require(raw != null && raw.length <= 18000)
        val data = JSONObject(raw)
        require(data.keys().asSequence().toSet() == setOf("schema", "path", "ticket", "complete", "items"))
        require(data.opt("schema") == SCHEMA)
        val path = data.getString("path")
        require(path == path(path) && data.opt("complete") is Boolean)
        val selection = data.getString("ticket")
        require(ticket.matches(selection))
        val rows = data.getJSONArray("items")
        require(rows.length() <= 100)
        val ids = mutableSetOf<String>()
        val links = (0 until rows.length()).map { index ->
            val row = rows.getJSONObject(index)
            require(row.keys().asSequence().toSet() == setOf("id", "createdAt"))
            val id = row.getString("id")
            require(uuid.matches(id) && ids.add(id))
            val at = if (row.isNull("createdAt")) null else row.getString("createdAt").also {
                require(it.length <= 40 && time.matches(it))
                Instant.parse(it)
            }
            Link(id, at)
        }
        Index(path, selection, data.getBoolean("complete"), links)
    }.getOrNull()

    fun detail(raw: String): String = if (parse(raw) != null) raw else "share_list_unconfirmed"
}

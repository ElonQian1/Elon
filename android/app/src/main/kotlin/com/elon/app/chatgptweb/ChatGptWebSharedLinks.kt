package com.elon.app.chatgptweb

import java.time.Instant
import org.json.JSONObject

/** Bound share selections only; never accepts arbitrary URLs or account headers. */
internal object ChatGptWebSharedLinks {
    private const val SCHEMA = "elon.conversation_shares.v1"
    private const val ACCOUNT_SCHEMA = "elon.account_shares.v1"
    private val uuid = Regex("[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")
    private val ticket = Regex("sl_[a-f0-9]{32}_[a-z0-9]{1,12}")
    private val time = Regex("\\d{4}-\\d{2}-\\d{2}T[0-9:.]+(?:Z|[+-]\\d{2}:\\d{2})")

    data class Link(val id: String, val createdAt: String?, val path: String? = null) {
        val url: String get() = "https://chatgpt.com/share/$id"
    }
    data class Index(val path: String, val ticket: String, val complete: Boolean, val items: List<Link>)
    data class AccountIndex(
        val ticket: String, val complete: Boolean, val offset: Int, val nextOffset: Int?, val items: List<Link>,
    )

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

    fun accountRequest(offset: Int, selection: String?): JSONObject? {
        if (offset !in 0..900 || offset % 100 != 0 || offset != 0 && selection == null ||
            selection != null && !ticket.matches(selection)) return null
        return JSONObject().put("operation", "list_account").put("offset", offset).apply {
            if (selection != null) put("ticket", selection)
        }
    }

    private fun links(data: JSONObject, account: Boolean = false): List<Link> {
        val rows = data.getJSONArray("items")
        require(rows.length() <= 100)
        val ids = mutableSetOf<String>()
        return (0 until rows.length()).map { index ->
            val row = rows.getJSONObject(index)
            require(row.keys().asSequence().toSet() == if (account) setOf("id", "path", "createdAt") else setOf("id", "createdAt"))
            val id = row.getString("id")
            require(uuid.matches(id) && ids.add(id))
            val at = if (row.isNull("createdAt")) null else row.getString("createdAt").also {
                require(it.length <= 40 && time.matches(it))
                Instant.parse(it)
            }
            val source = if (account) row.getString("path").also { require(it == path(it)) } else null
            Link(id, at, source)
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
        Index(path, selection, data.getBoolean("complete"), links(data))
    }.getOrNull()

    fun parseAccount(raw: String?): AccountIndex? = runCatching {
        require(raw != null && raw.length <= 18000)
        val data = JSONObject(raw)
        require(data.keys().asSequence().toSet() == setOf("schema", "ticket", "complete", "offset", "nextOffset", "items"))
        require(data.opt("schema") == ACCOUNT_SCHEMA && data.opt("complete") is Boolean)
        val selection = data.getString("ticket")
        require(ticket.matches(selection))
        val offset = data.opt("offset") as? Int ?: error("invalid_offset")
        require(offset in 0..900 && offset % 100 == 0)
        val next = if (data.isNull("nextOffset")) null else data.opt("nextOffset") as? Int ?: error("invalid_next_offset")
        val items = links(data, account = true)
        require(next == null || next == offset + 100 && next <= 900 && items.size == 100)
        require(offset == 0 || items.isNotEmpty())
        AccountIndex(selection, data.getBoolean("complete"), offset, next, items)
    }.getOrNull()

    fun detail(raw: String): String = if (parse(raw) != null || parseAccount(raw) != null) raw else "share_list_unconfirmed"
}

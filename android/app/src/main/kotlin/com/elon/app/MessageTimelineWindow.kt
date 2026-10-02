package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder

/** Native counterpart of elon.message_timeline.v1; pending sends remain in the outbox. */
internal class MessageTimelineWindow(private val maxMessages: Int = 150, private val maxBytes: Int = 1_500_000) {
    private var rows = listOf<JSONObject>()
    private var before: String? = null
    private var after: String? = null
    private var version = 1
    private var around = emptyMap<String, String>()
    private var beforeLocate: Triple<Int, Boolean, Boolean>? = null
    var target: JSONObject? = null; private set
    private var sync: String? = null
    var hasOlder = false; private set
    var hasNewer = false; private set
    private var readingHistory = false
    var following = true
        set(value) { field = value && !readingHistory }
    var moreChanges = false; private set
    var nextDirection = "sync"
    var lastRead: String? = null
    fun latest() { nextDirection = "latest" }
    fun older() { if (hasOlder) { nextDirection = "older"; following = false } }
    fun newer() { if (hasNewer && version == 2) { nextDirection = "newer"; following = false } }
    fun locate(query: Map<String, String>) { if (beforeLocate == null) beforeLocate = Triple(version, readingHistory, following); version = 2; readingHistory = true; around = query; nextDirection = "around"; following = false; target = null }
    fun cancelLocate() { beforeLocate?.let { version = it.first; readingHistory = it.second; following = it.third }; beforeLocate = null; around = emptyMap(); nextDirection = "sync"; target = null }
    fun direction(): String = nextDirection.let { if (it == "sync" && sync == null) "latest" else it }
    fun path(key: String, direction: String): String {
        val route = "/api/me/message-timeline" + if (version == 2) "/v2" else ""
        if (direction == "window") return "$route/window"
        fun encode(value: String) = URLEncoder.encode(value, "UTF-8")
        val base = "$route?kind=${encode(key.substringBefore(':'))}&id=${encode(key.substringAfter(':'))}&limit=50"
        return base + when (direction) {
            "older" -> before?.let { "&before=${encode(it)}" }.orEmpty()
            "newer" -> after?.let { "&after=${encode(it)}" }.orEmpty()
            "around" -> around.entries.joinToString("") { "&${encode(it.key)}=${encode(it.value)}" }
            "sync" -> sync?.let { "&sync=${encode(it)}" }.orEmpty()
            else -> ""
        }
    }
    fun accept(page: JSONObject, direction: String): JSONArray {
        require(page.optString("schema") in listOf("elon.message_timeline.v1", "elon.message_timeline.v2")) { "消息分页版本不匹配" }
        if (page.optString("schema") == "elon.message_timeline.v2") version = 2
        if (page.optBoolean("reset")) { nextDirection = if (following && !hasNewer) "latest" else "window"; moreChanges = true; return JSONArray(rows) }
        val incoming = page.getJSONArray("messages")
        val removed = page.getJSONArray("removed_ids")
        if (direction == "latest") { rows = emptyList(); hasNewer = false; readingHistory = false; following = true; beforeLocate = null }
        if (direction == "around") { rows = emptyList(); hasOlder = false; hasNewer = false; following = false; target = page.optJSONObject("target"); beforeLocate = null }
        if (direction == "window") { hasNewer = true; following = false }
        if (direction != "sync" && direction != "window") {
            if (version == 2) {
                if (direction != "newer") hasOlder = page.optBoolean("has_older")
                if (direction != "older") hasNewer = page.optBoolean("has_newer")
            } else hasOlder = page.optBoolean("has_more")
        }
        val existing = rows.associateBy { it.getString("id") }.toMutableMap()
        val first = rows.firstOrNull()
        repeat(removed.length()) { existing.remove(removed.getString(it)) }
        repeat(incoming.length()) { index ->
            val message = incoming.getJSONObject(index)
            val id = message.getString("id")
            val previous = existing[id]
            if (direction == "sync" && previous == null && first != null && compare(message, first) < 0) return@repeat
            if (direction == "sync" && previous == null && (!following || hasNewer)) { hasNewer = true; return@repeat }
            val oldWins = previous != null && message.optString("recalled_at").let { it.isBlank() || it == "null" } &&
                (previous.optString("recalled_at").let { it.isNotBlank() && it != "null" } || previous.optLong("revision", 1) > message.optLong("revision", 1))
            existing[id] = if (oldWins) previous!! else message
        }
        val bounded = existing.values.sortedWith(::compare).toMutableList()
        var bytes = bounded.sumOf { it.toString().toByteArray(Charsets.UTF_8).size }
        while (bounded.size > 1 && (bounded.size > maxMessages || bytes > maxBytes)) {
            val head = direction == "newer" || (direction != "older" && following)
            val removedRow = bounded.removeAt(if (head) 0 else bounded.lastIndex)
            bytes -= removedRow.toString().toByteArray(Charsets.UTF_8).size
            if (head) hasOlder = true else hasNewer = true
        }
        rows = bounded
        before = rows.firstOrNull()?.optString("timeline_cursor")?.takeIf { it.isNotBlank() }
        after = rows.lastOrNull()?.optString("timeline_after_cursor")?.takeIf { it.isNotBlank() }
        page.optString("sync").takeIf { it.isNotBlank() && it != "null" }?.let { sync = it }
        moreChanges = direction == "sync" && page.optBoolean("has_more")
        nextDirection = "sync"
        return JSONArray(rows)
    }
    fun readReceipt(key: String): JSONObject? {
        val id = rows.lastOrNull()?.optString("id") ?: return null
        if (!following || hasNewer || lastRead == id) return null
        return JSONObject().put("kind", key.substringBefore(':')).put("id", key.substringAfter(':')).put("message_id", id)
    }
    fun windowRequest(key: String) = JSONObject().put("kind", key.substringBefore(':')).put("id", key.substringAfter(':'))
        .put("message_ids", JSONArray(rows.map { it.getString("id") }))
    companion object {
        private fun compare(a: JSONObject, b: JSONObject): Int = a.getString("created_at").compareTo(b.getString("created_at"))
            .takeIf { it != 0 } ?: a.getString("id").compareTo(b.getString("id"))
    }
}

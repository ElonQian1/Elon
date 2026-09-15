package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

/** Protect disk snapshots as well as the visible adapter against stale revisions. */
internal fun protectSocialChatRows(current: List<ChatMessage>, rows: JSONArray): JSONArray {
    val known = current.filter { !it.id.isNullOrBlank() }.associateBy { it.id }
    return JSONArray().apply {
        repeat(rows.length()) { index ->
            val row = JSONObject(rows.getJSONObject(index).toString())
            val previous = known[row.optString("id")]
            if (previous != null && (!previous.recalledAt.isNullOrBlank() || previous.revision > row.optLong("revision", 1))) {
                row.put("content", previous.content).put("revision", previous.revision).put("edited_at", previous.editedAt)
                if (!previous.recalledAt.isNullOrBlank()) row.put("recalled_at", previous.recalledAt).put("attachments", JSONArray())
            }
            put(row)
        }
    }
}

internal fun mergeSocialChatMessages(current: List<ChatMessage>, incoming: List<ChatMessage>): List<ChatMessage> {
    val confirmed = mergeGroupMessageRevisions(current, incoming).distinctBy { it.id }
    // Receiving messages must remain independent from unresolved or failed outgoing sends.
    return confirmed + current.filter { it.id.isNullOrBlank() && !it.sendStatus.isNullOrBlank() }
}

internal fun completeSocialChatSend(messages: MutableList<ChatMessage>, pending: ChatMessage, sent: ChatMessage) {
    if (sent.id.isNullOrBlank()) {
        pending.sendStatus = "发送结果未确认，请同步并核对后再决定是否重试"
        return
    }
    val authoritative = messages.firstOrNull { it.id == sent.id } ?: sent
    messages.removeAll { it !== pending && it.id == sent.id }
    val index = messages.indexOfFirst { it === pending }
    if (index >= 0) messages[index] = authoritative else messages.add(authoritative)
}

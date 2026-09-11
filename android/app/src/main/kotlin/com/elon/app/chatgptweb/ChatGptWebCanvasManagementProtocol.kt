package com.elon.app.chatgptweb

import org.json.JSONObject

internal data class ChatGptWebCanvasHistory(val documentId: String, val ticket: String, val beforeVersion: Long,
    val nextBeforeVersion: Long?, val versions: List<ChatGptWebCanvasDocument>) {
    override fun toString() = "CanvasHistory(count=${versions.size})"
}

internal data class ChatGptWebCanvasShare(val documentId: String, val state: String, val id: String, val documentVersion: Long?) {
    val url: String? get() = if (state == "public") "https://chatgpt.com/canvas/shared/$id" else null
    override fun toString() = "CanvasShare(state=$state)"
}

internal object ChatGptWebCanvasManagementProtocol {
    private fun number(value: Any?): Long {
        require(value is Int || value is Long)
        return (value as Number).toLong().also { require(it in 1..9_007_199_254_740_991L) }
    }
    fun history(value: JSONObject, documents: List<ChatGptWebCanvasDocument>): ChatGptWebCanvasHistory {
        require(value.keys().asSequence().toSet() == setOf("documentId", "ticket", "beforeVersion", "nextBeforeVersion", "versions"))
        val id = value.getString("documentId")
        val current = documents.single { it.id == id }
        val ticket = value.getString("ticket")
        require(Regex("cd_[a-f0-9]{32}_[a-z0-9]{1,12}").matches(ticket))
        val before = number(value.opt("beforeVersion"))
        require(before <= current.documentVersion)
        val rows = value.getJSONArray("versions")
        require(rows.length() <= 200)
        var previous = before
        val versions = (0 until rows.length()).map {
            ChatGptWebCanvasDocumentProtocol.parseDocument(rows.getJSONObject(it)).also { document ->
                require(document.id == id && document.documentVersion < previous)
                previous = document.documentVersion
            }
        }
        val next = if (value.isNull("nextBeforeVersion")) null else number(value.opt("nextBeforeVersion"))
        require(next == versions.lastOrNull()?.documentVersion?.takeIf { it > 1 })
        return ChatGptWebCanvasHistory(id, ticket, before, next, versions)
    }
    fun share(value: JSONObject, documents: List<ChatGptWebCanvasDocument>): ChatGptWebCanvasShare {
        require(value.keys().asSequence().toSet() == setOf("documentId", "state", "id", "documentVersion"))
        val documentId = value.getString("documentId")
        require(documents.any { it.id == documentId })
        val state = value.getString("state")
        val id = value.getString("id")
        require(state in setOf("missing", "public", "restricted"))
        val version = if (value.isNull("documentVersion")) null else number(value.opt("documentVersion"))
        if (state == "public") require(version != null && Regex("[A-Za-z0-9_-]{1,128}").matches(id))
        else require(id.isEmpty() && version == null)
        return ChatGptWebCanvasShare(documentId, state, id, version)
    }
}

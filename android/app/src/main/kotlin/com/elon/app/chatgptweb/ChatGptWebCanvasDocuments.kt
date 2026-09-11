package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal data class ChatGptWebCanvasComment(val id: String, val start: Int, val end: Int, val content: String) {
    fun json() = JSONObject().put("id", id).put("start", start).put("end", end).put("content", content)
    override fun toString() = "CanvasComment(range=$start..$end)"
}

internal data class ChatGptWebCanvasDocument(
    val id: String, val title: String, val content: String, val documentType: String,
    val documentVersion: Long, val comments: List<ChatGptWebCanvasComment>,
) {
    override fun toString() = "CanvasDocument(version=$documentVersion,length=${content.length})"
}

internal data class ChatGptWebCanvasDocuments(
    val requestId: String, val path: String, val ticket: String, val scope: String,
    val documents: List<ChatGptWebCanvasDocument>, val unconfirmedWrite: Boolean,
) {
    override fun toString() = "CanvasDocuments(count=${documents.size},unconfirmed=$unconfirmedWrite)"
    fun diagnostic() = JSONObject().put("request_id", requestId).put("document_count", documents.size)
        .put("unconfirmed_write", unconfirmedWrite)
}

internal object ChatGptWebCanvasDocumentProtocol {
    const val ACTION = "canvas_document"
    const val MAX_CONTENT = 128 * 1024
    private val id = Regex("[A-Za-z0-9_-]{1,128}")
    private val token = Regex("cd_[a-f0-9]{32}_[a-z0-9]{1,12}")
    private val requestId = Regex("mcp_[a-z0-9]{1,32}")
    private val type = Regex("(?:document|webview|code/[a-z0-9+#._-]{1,40})")
    private val path = Regex("/(?:g/[A-Za-z0-9_-]{1,200}/)?c/[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")

    fun validPath(value: String) = value.length <= 256 && path.matches(value)
    fun boundary(text: String, offset: Int): Boolean = offset in 0..text.length &&
        (offset == 0 || offset == text.length || !(text[offset].isLowSurrogate() && text[offset - 1].isHighSurrogate()))

    private fun text(value: Any?): String {
        val content = value as? String ?: error("content_type")
        require(content.length <= MAX_CONTENT)
        var index = 0
        while (index < content.length) {
            val char = content[index++]
            if (char.isHighSurrogate()) require(index < content.length && content[index++].isLowSurrogate())
            else require(!char.isLowSurrogate())
        }
        return content
    }

    fun parseComments(rows: JSONArray, content: String): List<ChatGptWebCanvasComment> {
        require(rows.length() <= 1000)
        val comments = (0 until rows.length()).map { index ->
            val row = rows.getJSONObject(index)
            require(row.keys().asSequence().toSet() == setOf("id", "content", "start", "end"))
            val identifier = row.opt("id") as? String ?: error("comment_id")
            val body = row.opt("content") as? String ?: error("comment_content")
            val start = integer(row.opt("start"), 0L..content.length.toLong()).toInt()
            val end = integer(row.opt("end"), start.toLong()..content.length.toLong()).toInt()
            require(id.matches(identifier) && body.length <= 16384 && boundary(content, start) && boundary(content, end))
            ChatGptWebCanvasComment(identifier, start, end, body)
        }
        require(comments.distinctBy { it.id }.size == comments.size)
        return comments
    }

    fun parse(value: JSONObject): ChatGptWebCanvasDocuments? = runCatching {
        require(value.toString().length <= 2 * 1024 * 1024)
        require(value.keys().asSequence().toSet() == setOf("type", "version", "requestId", "path", "ticket", "scope", "documents", "unconfirmedWrite"))
        require(value.opt("type") == "canvas_documents" && value.opt("version") == 1 && value.opt("unconfirmedWrite") is Boolean)
        val req = value.getString("requestId"); val owner = value.getString("path")
        val selected = value.getString("ticket"); val scope = value.getString("scope")
        require(requestId.matches(req) && validPath(owner) && token.matches(selected) && token.matches(scope))
        val rows = value.getJSONArray("documents")
        require(rows.length() <= 200)
        val documents = (0 until rows.length()).map { index ->
            val row = rows.getJSONObject(index)
            require(row.keys().asSequence().toSet() == setOf("id", "title", "content", "documentType", "documentVersion", "comments"))
            val identifier = row.getString("id"); val title = row.getString("title"); val kind = row.getString("documentType")
            require(id.matches(identifier) && !identifier.startsWith("temp-") && type.matches(kind))
            require(title.isNotBlank() && title.length <= 512 && title.none { it.code < 32 || it.code == 127 })
            val content = text(row.opt("content"))
            ChatGptWebCanvasDocument(identifier, title, content, kind,
                integer(row.opt("documentVersion"), 1..9_007_199_254_740_991L), parseComments(row.getJSONArray("comments"), content))
        }
        require(documents.distinctBy { it.id }.size == documents.size)
        ChatGptWebCanvasDocuments(req, owner, selected, scope, documents, value.getBoolean("unconfirmedWrite"))
    }.getOrNull()

    fun request(value: JSONObject): JSONObject? = runCatching {
        require(value.toString().length <= 2 * 1024 * 1024)
        val operation = value.opt("operation") as? String ?: error("operation")
        val owner = value.opt("path") as? String ?: error("path")
        require(validPath(owner))
        val keys = when (operation) {
            "list" -> setOf("operation", "path", "force")
            "save" -> setOf("operation", "path", "ticket", "scope", "id", "content", "comments")
            "verify" -> setOf("operation", "path", "ticket", "scope", "id")
            else -> error("operation")
        }
        require(value.keys().asSequence().toSet() == keys)
        if (operation == "list") require(value.opt("force") is Boolean)
        else {
            require(token.matches(value.opt("ticket") as? String ?: "") && token.matches(value.opt("scope") as? String ?: ""))
            require(id.matches(value.opt("id") as? String ?: ""))
            if (operation == "save") parseComments(value.getJSONArray("comments"), text(value.opt("content")))
        }
        JSONObject(value.toString())
    }.getOrNull()

    fun dispatch(args: JSONObject, commands: ChatGptWebMcpCommandPort, dispatch: (String, (String) -> Unit) -> Unit): String? {
        val request = args.optJSONObject("canvas_request")?.let(::request) ?: return "canvas_request_invalid"
        val confirmed = args.opt("user_confirmed") as? Boolean ?: return "canvas_confirmation_required"
        if (request.getString("operation") == "save" && !confirmed) return "canvas_confirmation_required"
        dispatch(ACTION) { commands.canvasDocument(request, confirmed, it) }
        return null
    }

    fun detail(raw: String) = raw.takeIf { Regex("canvas_(?:[a-z_]{1,64}|http_[0-9]{3})").matches(it) } ?: "canvas_unavailable"
    private fun integer(value: Any?, range: LongRange): Long {
        require(value is Int || value is Long)
        return (value as Number).toLong().also { require(it in range) }
    }
}

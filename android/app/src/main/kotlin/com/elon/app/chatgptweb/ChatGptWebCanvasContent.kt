package com.elon.app.chatgptweb

import org.json.JSONObject

/** Display data stays outside command receipts and MCP diagnostics. Never executes document code. */
internal data class ChatGptWebCanvasContent(
    val requestId: String,
    val id: String,
    val title: String,
    val content: String,
    val documentType: String,
    val documentVersion: Long?,
) {
    val isCode: Boolean get() = documentType != "document"

    override fun toString(): String = "CanvasContent(version=$documentVersion, length=${content.length})"

    fun diagnostic(): JSONObject = JSONObject().put("request_id", requestId)
        .put("content_length", content.length).put("document_type", documentType)
        .put("document_version", documentVersion ?: JSONObject.NULL)

    companion object {
        const val ACTION = "share_conversation"
        private val id = Regex("[A-Za-z0-9_-]{1,128}")
        private val ticket = Regex("sl_[a-f0-9]{32}_[a-z0-9]{1,12}")
        private val requestId = Regex("mcp_[a-z0-9]{1,32}")
        private val type = Regex("(?:document|webview|code/[a-z0-9+#._-]{1,40})")
        private val keys = setOf("type", "version", "requestId", "id", "title", "content",
            "documentType", "documentVersion", "access")

        fun request(id: String, selection: String): JSONObject? {
            if (!this.id.matches(id) || !ticket.matches(selection)) return null
            return JSONObject().put("operation", "read_account").put("resource", "canvas")
                .put("id", id).put("ticket", selection)
        }

        fun parse(value: JSONObject): ChatGptWebCanvasContent? = runCatching {
            require(value.keys().asSequence().toSet() == keys)
            require(value.opt("type") == "canvas_shared_content" && value.opt("version") == 1)
            require(value.opt("access") == "public")
            val request = value.opt("requestId") as? String ?: error("request_type")
            val selected = value.opt("id") as? String ?: error("id_type")
            val title = value.opt("title") as? String ?: error("title_type")
            val body = value.opt("content") as? String ?: error("content_type")
            val kind = value.opt("documentType") as? String ?: error("document_type")
            val version = value.opt("documentVersion")
            require(requestId.matches(request) && id.matches(selected) && type.matches(kind))
            require(title.isNotBlank() && title.length <= 512 && title.none { it.code < 32 || it.code == 127 })
            require(body.length <= 128 * 1024)
            require(version == JSONObject.NULL || version is Int || version is Long)
            val number = if (version == JSONObject.NULL) null else (version as Number).toLong()
            require(number == null || number in 1L..9_007_199_254_740_991L)
            ChatGptWebCanvasContent(request, selected, title, body, kind, number)
        }.getOrNull()
    }
}

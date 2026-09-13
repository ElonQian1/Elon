package com.elon.app.chatgptweb

import com.elon.app.WebChatTextBlock
import org.json.JSONObject

internal data class ChatGptWebWritingBlock(
    val requestId: String, val path: String, val ticket: String,
    val id: String, val messageId: String, val pending: Boolean,
) {
    override fun toString() = "WritingBlock(pending=$pending)"
}

internal object ChatGptWebWritingBlockProtocol {
    const val ACTION = "writing_block"
    private val identifier = Regex("[A-Za-z0-9_-]{1,128}")
    private val ticket = Regex("wb_[a-f0-9]{32}")
    private val path = Regex(
        "(?:/g/g-p-[a-fA-F0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?" +
            "/c/[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}",
    )

    fun parse(value: JSONObject): ChatGptWebWritingBlock? = runCatching {
        require(value.keys().asSequence().toSet() == setOf("type", "version", "requestId", "path", "ticket", "id", "messageId", "pending"))
        require(value.opt("type") == ACTION && value.opt("version") == 1 && value.opt("pending") is Boolean)
        require(Regex("mcp_[a-z0-9]{1,32}").matches(value.getString("requestId")))
        require(path.matches(value.getString("path")) && ticket.matches(value.getString("ticket")))
        require(identifier.matches(value.getString("id")) && identifier.matches(value.getString("messageId")))
        ChatGptWebWritingBlock(value.getString("requestId"), value.getString("path"), value.getString("ticket"),
            value.getString("id"), value.getString("messageId"), value.getBoolean("pending"))
    }.getOrNull()

    fun request(value: JSONObject): JSONObject? = runCatching {
        require(value.toString().length <= 900_000 && path.matches(value.getString("path")))
        val operation = value.getString("operation")
        val keys = when (operation) {
            "prepare" -> setOf("operation", "path", "messageId", "id", "content")
            "save" -> setOf("operation", "path", "ticket", "content")
            "verify" -> setOf("operation", "path", "ticket")
            else -> error("operation")
        }
        require(value.keys().asSequence().toSet() == keys)
        if (operation == "prepare") require(identifier.matches(value.getString("id")) && identifier.matches(value.getString("messageId")))
        else require(ticket.matches(value.getString("ticket")))
        if (operation != "verify") {
            val content = value.opt("content") as? String ?: error("content")
            require(content.length <= WebChatTextBlock.MAX_CONTENT)
            var index = 0
            while (index < content.length) {
                val c = content[index++]
                if (c.isHighSurrogate()) require(index < content.length && content[index++].isLowSurrogate())
                else require(!c.isLowSurrogate())
            }
        }
        JSONObject(value.toString())
    }.getOrNull()

    fun dispatch(args: JSONObject, commands: ChatGptWebMcpCommandPort, dispatch: (String, (String) -> Unit) -> Unit): String? {
        val request = args.optJSONObject("writing_request")?.let(::request) ?: return "writing_request_invalid"
        val confirmed = args.opt("user_confirmed") as? Boolean ?: return "writing_confirmation_required"
        if (request.getString("operation") == "save" && !confirmed) return "writing_confirmation_required"
        dispatch(ACTION) { commands.writingBlock(request, confirmed, it) }
        return null
    }
}

package com.elon.app.socialquotes

import com.elon.app.ChatAttachment
import com.elon.app.ChatMessage
import com.elon.app.chatAttachmentsFromJsonArray
import org.json.JSONObject

data class SocialQuote(
    val messageId: String,
    val senderName: String,
    val content: String,
    val attachments: List<ChatAttachment> = emptyList(),
    val revision: Long = 1,
    val unavailable: Boolean = false,
) {
    fun source(): JSONObject = JSONObject().put("message_id", messageId).put("revision", revision)
}

internal object SocialQuoteCodec {
    private val legacy = Regex("^(>[^\\n]*(?:\\r?\\n>[^\\n]*)*)\\r?\\n(?:[ \\t]*\\r?\\n)?([^>][\\s\\S]*)$")
    private val header = Regex("^引用 (.+?)(?: · 第 (\\d+) 版)?$")

    fun split(content: String): Pair<String, SocialQuote?> {
        val match = legacy.matchEntire(content) ?: return content to null
        val lines = match.groupValues[1].lines().map { it.replace(Regex("^(?:>\\s*)+"), "") }.toMutableList()
        val name = header.matchEntire(lines.first())
        if (name != null) lines.removeAt(0)
        return match.groupValues[2] to SocialQuote("", name?.groupValues?.get(1).orEmpty(), lines.joinToString("\n"))
    }

    fun bind(message: ChatMessage, json: JSONObject): ChatMessage = message.apply {
        val value = json.optJSONObject("quote")
        if (value != null) {
            quote = SocialQuote(value.optString("message_id"), value.optString("sender_name"), value.optString("content"),
                chatAttachmentsFromJsonArray(value.optJSONArray("attachments")), value.optLong("revision", 1), value.optBoolean("unavailable"))
        } else if (role != "ai") {
            val parsed = split(content)
            content = parsed.first; quote = parsed.second
        }
    }

    fun from(message: ChatMessage, ownName: String): SocialQuote = SocialQuote(message.id.orEmpty(),
        message.senderLabel?.takeIf { it.isNotBlank() } ?: ownName, split(message.content).first,
        message.attachments.orEmpty(), message.revision, !message.recalledAt.isNullOrBlank())

    fun summary(quote: SocialQuote): String {
        if (quote.unavailable) return "原消息已撤回或不可用"
        val content = split(quote.content).first.trim()
        com.elon.app.chatrecords.ChatRecordDocument.card(content)?.let { return "[聊天记录] ${it.optString("title", "聊天记录")}" }
        if (content.isNotBlank()) return content
        val attachment = quote.attachments.firstOrNull() ?: return "[消息]"
        val kind = when { attachment.isImage() -> "图片"; attachment.isVoice() -> "语音"; attachment.kind == "video" -> "视频"; else -> "文件" }
        return "[$kind] ${attachment.displayName ?: attachment.fileName.orEmpty()}"
    }
}

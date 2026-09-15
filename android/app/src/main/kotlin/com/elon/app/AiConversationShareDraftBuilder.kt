package com.elon.app

import org.commonmark.node.AbstractVisitor
import org.commonmark.node.Code
import org.commonmark.node.FencedCodeBlock
import org.commonmark.node.HardLineBreak
import org.commonmark.node.IndentedCodeBlock
import org.commonmark.node.SoftLineBreak
import org.commonmark.node.Text
import org.commonmark.parser.Parser

/** Freeze only the explicitly selected rows; neither title nor excerpt reads hidden context. */
internal object AiConversationShareDraftBuilder {
    const val MAX_MESSAGES = 200
    const val MAX_MESSAGE_CHARS = 120_000
    const val MAX_TOTAL_CHARS = 500_000

    fun build(current: List<ChatMessage>, selected: List<ChatMessage>, streaming: Boolean): AiConversationShareDraft {
        require(!streaming) { "请等待当前回复结束后再分享" }
        require(selected.size in 1..MAX_MESSAGES) { "一次可分享 1 至 $MAX_MESSAGES 条消息" }
        val indices = selected.map { chosen ->
            val id = chosen.id?.takeIf(String::isNotBlank) ?: error("消息尚未同步，请稍后再选择")
            val matches = current.withIndex().filter { it.value.id == id }
            require(matches.size == 1) { "消息列表已变化，请重新选择" }
            val indexed = matches.single()
            require(indexed.value.revision == chosen.revision && indexed.value.content == chosen.content &&
                indexed.value.webChatMessage == chosen.webChatMessage && indexed.value.attachments == chosen.attachments) {
                "所选消息已更新，请重新选择"
            }
            indexed.index
        }.sorted()
        require(indices.distinct().size == indices.size) { "不能重复选择同一条消息" }
        val provider = current[indices.first()].id!!.substringBefore(':')
        require(provider in setOf("chatgpt_web", "google_web")) { "请在网页 AI 会话中选择要分享的消息" }
        var total = 0L
        val frozen = indices.map { index ->
            val message = current[index]
            require(message.id!!.startsWith("$provider:") && message.role in setOf("user", "friend")) {
                "所选消息不属于同一个 AI 会话"
            }
            require(message.id!!.substringAfter(':') !in setOf("status", "streaming", "pending_user") &&
                message.sendStatus.isNullOrBlank() && !message.isRecalled()) { "待发送或状态消息不能分享" }
            require(message.content.length <= MAX_MESSAGE_CHARS) { "单条消息过长，请减少分享内容" }
            total += message.content.length
            message.webChatMessage?.contentParts.orEmpty().forEach { part ->
                part.textBlock?.let { block ->
                    require(block.complete) { "写作块尚未完成，请稍后再分享" }
                    total += block.content.length
                }
            }
            require(total <= MAX_TOTAL_CHARS) { "所选内容过长，请分次分享" }
            message.copy(
                attachments = message.attachments?.map { it.copy(annotations = it.annotations.toList()) },
                webChatMessage = message.webChatMessage?.copy(
                    actions = emptySet(), contentParts = message.webChatMessage!!.contentParts.toList(),
                ),
                evidenceTitle = null, evidenceDetails = null, apkUrl = null, codexThreadUri = null,
            )
        }
        val excerpt = excerpt(frozen)
        return AiConversationShareDraft(
            provider = if (provider == "chatgpt_web") "chatgpt" else "google",
            title = excerpt.lineSequence().firstOrNull()?.take(60).orEmpty().ifBlank { "AI 聊天记录" },
            summary = excerpt.take(240), messages = frozen,
            gaps = indices.indices.filter { it > 0 && indices[it] > indices[it - 1] + 1 }.toSet(),
        )
    }

    fun excerpt(messages: List<ChatMessage>): String = messages.asSequence().map { message ->
        val body = message.content.ifBlank {
            message.webChatMessage?.contentParts?.firstNotNullOfOrNull { it.textBlock?.content }.orEmpty()
        }
        plainText(body).ifBlank {
            if (message.attachments.orEmpty().any(ChatAttachment::isImage) ||
                message.webChatMessage?.contentParts.orEmpty().any { it.type == "image" }) "[图片]" else "[附件]"
        }.take(120)
    }.take(3).joinToString("\n").take(240)

    private fun plainText(markdown: String): String {
        val text = StringBuilder()
        Parser.builder().build().parse(markdown.take(MAX_MESSAGE_CHARS)).accept(object : AbstractVisitor() {
            override fun visit(node: Text) { text.append(node.literal).append(' ') }
            override fun visit(node: Code) { text.append(node.literal).append(' ') }
            override fun visit(node: FencedCodeBlock) { text.append(node.literal).append(' ') }
            override fun visit(node: IndentedCodeBlock) { text.append(node.literal).append(' ') }
            override fun visit(node: SoftLineBreak) { text.append(' ') }
            override fun visit(node: HardLineBreak) { text.append(' ') }
        })
        return text.toString().replace(Regex("\\s+"), " ").trim()
    }
}

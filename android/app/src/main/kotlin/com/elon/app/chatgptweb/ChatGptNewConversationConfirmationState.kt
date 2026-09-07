package com.elon.app.chatgptweb

internal class ChatGptNewConversationConfirmationState(private val nowMs: () -> Long) {
    private var current: String? = null
    private var lastTicket: String? = null
    private var expiresAt = 0L

    fun offer(action: String, ok: Boolean, detail: String): String? {
        if (ok) return null
        val receipt = parse(action, detail) ?: return null
        val ticket = receipt.second ?: return null
        if (current != null || ticket == lastTicket) return null
        current = ticket
        lastTicket = ticket
        expiresAt = nowMs() + TIMEOUT_MS
        return ticket
    }

    fun consume(ticket: String, requireFresh: Boolean = true): Boolean {
        if (current != ticket) return false
        current = null
        return !requireFresh || nowMs() < expiresAt
    }

    companion object {
        const val TIMEOUT_MS = 50_000L
        private val RECEIPT = Regex(
            "\\[runtime_new_chat:(ready|confirmation_required|cancelled|confirmation_expired|context_changed|invocation_unconfirmed|timeout|busy)](?: \\[confirmation_id:([a-f0-9]{32})])?$",
        )

        private fun parse(action: String, detail: String): Pair<String, String?>? {
            if (action != "new_conversation" || detail.length > 512) return null
            val match = RECEIPT.find(detail) ?: return null
            val ticket = match.groupValues[2].takeIf(String::isNotEmpty)
            if (ticket != null && match.groupValues[1] != "confirmation_required") return null
            return match.groupValues[1] to ticket
        }

        fun userDetail(action: String, detail: String): String? = when (parse(action, detail)?.first) {
            "ready" -> "新会话已就绪。"
            "confirmation_required" -> "新建访客聊天前，需要确认是否清除当前未保存的会话。"
            "cancelled" -> "已保留当前聊天。"
            "confirmation_expired" -> "确认已过期，请重新点击新会话。"
            "context_changed" -> "会话或草稿已经变化，当前内容已保留，请重新确认。"
            "invocation_unconfirmed", "timeout" -> "新会话切换尚未确认，请稍后检查。"
            "busy" -> "请先处理当前的新会话操作。"
            else -> null
        }
    }
}

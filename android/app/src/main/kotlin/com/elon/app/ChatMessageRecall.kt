package com.elon.app

import java.time.Instant

internal const val MESSAGE_RECALL_WINDOW_MS = 60_000L

internal fun ChatMessage.isRecalled(): Boolean = !recalledAt.isNullOrBlank()

internal fun ChatMessage.canRecallNow(nowMs: Long = System.currentTimeMillis()): Boolean {
    if (role != "user" || isRecalled()) return false
    if (createdAtMs <= 0L) return false
    return nowMs - createdAtMs <= MESSAGE_RECALL_WINDOW_MS
}

internal fun ChatMessage.recallNoticeText(): String {
    if (role == "user") return "你撤回了一条消息"
    val sender = senderLabel?.trim().orEmpty()
    return if (sender.isBlank()) "对方撤回了一条消息" else "$sender 撤回了一条消息"
}

internal fun markChatMessageRecalled(message: ChatMessage, recalledBy: String? = null) {
    message.content = ""
    message.attachments = null
    message.projectPostCard = null
    message.apkUrl = null
    message.codexThreadUri = null
    message.evidenceTitle = null
    message.evidenceDetails = null
    message.suggestionStatus = null
    message.suggestionResolvedByName = null
    message.suggestionResolvedAt = null
    message.canResolveSuggestion = false
    message.finalReply = false
    message.sendStatus = null
    message.recalledAt = message.recalledAt ?: Instant.now().toString()
    message.recalledBy = message.recalledBy ?: recalledBy
}

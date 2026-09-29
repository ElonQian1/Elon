package com.elon.app

internal class ChatMessageContentActions(message: ChatMessage, text: String) {
    val isRecord = com.elon.app.chatrecords.ChatRecordDocument.card(message.content) != null
    val hasText = text.isNotBlank() && !isRecord
    private val available = !message.isRecalled()
    private val hasAttachments = !message.attachments.isNullOrEmpty()
    val canCopy = available && (hasText || hasAttachments)
    val canForward = available && (hasText || hasAttachments || !message.webChatMessage?.contentParts.isNullOrEmpty())
    val canAnalyze = available && (hasText || isRecord || hasAttachments)
    val images = if (available) message.attachments.orEmpty().filter(ChatAttachment::isImage) else emptyList()
}

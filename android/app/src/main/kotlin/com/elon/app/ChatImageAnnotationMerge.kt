package com.elon.app

internal fun List<ChatAttachment>.withMissingImageAnnotationsFrom(
    fallbackAttachments: List<ChatAttachment>?
): List<ChatAttachment> {
    if (isEmpty() || fallbackAttachments.isNullOrEmpty()) return this
    var changed = false
    val merged = mapIndexed { index, attachment ->
        val fallback = fallbackAttachments.getOrNull(index)
        if (attachment.shouldUseFallbackAnnotations(fallback)) {
            changed = true
            attachment.copy(annotations = fallback!!.annotations)
        } else {
            attachment
        }
    }
    return if (changed) merged else this
}

internal fun ChatMessage.withMissingImageAnnotationsFrom(
    fallbackAttachments: List<ChatAttachment>?
): ChatMessage {
    val currentAttachments = attachments ?: return this
    val merged = currentAttachments.withMissingImageAnnotationsFrom(fallbackAttachments)
    if (merged !== currentAttachments) {
        attachments = merged
    }
    return this
}

internal fun List<ChatMessage>.withMissingImageAnnotationsFromCurrent(
    currentMessages: List<ChatMessage>
): List<ChatMessage> {
    if (isEmpty() || currentMessages.isEmpty()) return this
    val currentById = currentMessages
        .mapNotNull { message ->
            val id = message.id?.trim()?.takeIf { it.isNotEmpty() } ?: return@mapNotNull null
            id to message
        }
        .toMap()
    return mapIndexed { index, incoming ->
        val incomingId = incoming.id?.trim()?.takeIf { it.isNotEmpty() }
        val fallback = incomingId?.let { currentById[it] }
            ?: currentMessages.getOrNull(index)?.takeIf { current ->
                current.id.isNullOrBlank() &&
                    current.role == incoming.role &&
                    current.content == incoming.content
            }
        incoming.withMissingImageAnnotationsFrom(fallback?.attachments)
    }
}

private fun ChatAttachment.shouldUseFallbackAnnotations(fallback: ChatAttachment?): Boolean {
    if (!isImage() || annotations.any { it.hasNote() }) return false
    if (fallback == null || !fallback.isImage()) return false
    return fallback.annotations.any { it.hasNote() }
}

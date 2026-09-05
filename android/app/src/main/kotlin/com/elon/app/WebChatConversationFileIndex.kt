package com.elon.app

internal data class WebChatConversationFile(
    val id: String,
    val messageId: String,
    val name: String,
    val kind: String,
    val role: String,
    val mediaType: String,
)

internal data class WebChatConversationFileIndex(
    val path: String,
    val requestId: String,
    val files: List<WebChatConversationFile>,
    val truncated: Boolean,
    val savedAtMs: Long = 0,
) {
    fun isFresh(nowMs: Long): Boolean = savedAtMs > 0 && nowMs - savedAtMs in 0 until 60_000
}

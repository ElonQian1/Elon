package com.elon.app

internal data class WebChatAttachmentProgress(
    val phase: String,
    val totalCount: Int,
    val completedCount: Int,
)

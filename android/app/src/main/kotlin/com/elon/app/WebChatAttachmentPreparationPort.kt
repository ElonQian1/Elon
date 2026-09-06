package com.elon.app

internal enum class WebChatAttachmentSelectionKind(val wireValue: String) {
    IMAGE("image"), DOCUMENT("document"),
}

internal fun interface WebChatAttachmentPreparationPort {
    fun begin(kind: WebChatAttachmentSelectionKind): WebChatAttachmentSelection?
}

internal interface WebChatAttachmentSelection {
    fun selected(attachments: List<PendingAttachment>)
    fun cancel()
}

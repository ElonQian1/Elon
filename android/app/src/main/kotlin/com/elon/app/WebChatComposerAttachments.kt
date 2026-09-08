package com.elon.app

internal data class WebChatComposerAttachment(
    val id: String,
    val name: String,
    val state: String,
    val removable: Boolean,
)

internal object WebChatComposerAttachments {
    fun visible(state: WebChatConsumerState?): List<WebChatComposerAttachment> =
        state?.takeIf { it.adapterCurrent && it.pageUrl.isNotBlank() }
            ?.attachments.orEmpty().filter { it.id.isNotBlank() }.distinctBy { it.id }

    fun canRemove(page: String, item: WebChatComposerAttachment, state: WebChatConsumerState?): Boolean =
        state != null && state.pageUrl == page && !state.streaming &&
            visible(state).any { it.id == item.id && it.removable }

    fun status(value: String): String = when (value) {
        "ready" -> "已添加"
        "uploading" -> "上传中"
        "error", "failed" -> "上传失败"
        else -> "处理中"
    }
}

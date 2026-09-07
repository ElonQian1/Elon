package com.elon.app

import android.view.View
import android.widget.PopupMenu
import com.elon.app.chatgptweb.ChatGptWebNativeAttachmentPolicy

internal class WebChatAttachmentUploadOptions(
    private val controller: WebChatSocialController,
    private val selectionCount: () -> Int,
) : WebChatAttachmentPreparationPort {
    override fun begin(kind: WebChatAttachmentSelectionKind) = controller.beginAttachmentSelection(kind)

    override fun showUploadOptions(
        anchor: View,
        attachment: PendingAttachment,
        onSelected: (Boolean) -> Unit,
    ): Boolean {
        if (!available() || !anchor.isAttachedToWindow) return false
        fun copySupported() = selectionCount() == 1 && ChatGptWebNativeAttachmentPolicy.supports(
            attachment.mimeType, attachment.file.length(), attachment.imageWidth, attachment.imageHeight,
        )
        if (!copySupported() && !attachment.chatGptUploadCopy) return false
        val conversation = controller.currentConversationPath()
        PopupMenu(anchor.context, anchor).apply {
            menu.add(0, REUSE, 0, "优先复用已有文件").isChecked = !attachment.chatGptUploadCopy
            menu.add(0, COPY, 1, "重新上传一份").apply {
                isChecked = attachment.chatGptUploadCopy
                isEnabled = copySupported()
            }
            menu.setGroupCheckable(0, true, true)
            setOnMenuItemClickListener { item ->
                if (!available() || !anchor.isAttachedToWindow ||
                    controller.currentConversationPath() != conversation
                ) return@setOnMenuItemClickListener false
                when (item.itemId) {
                    REUSE -> onSelected(false)
                    COPY -> if (copySupported()) onSelected(true) else return@setOnMenuItemClickListener false
                    else -> return@setOnMenuItemClickListener false
                }
                true
            }
            show()
        }
        return true
    }

    private fun available() = controller.providerId == WebChatProviderId.CHATGPT_WEB &&
        controller.isActive() && !controller.streaming() && controller.pendingAttachmentCount() == 0

    private companion object {
        const val REUSE = 1
        const val COPY = 2
    }
}

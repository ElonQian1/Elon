package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptBackgroundSession

internal class ChatGptSocialImageContentController(
    private val activity: AppCompatActivity,
    private val session: ChatGptBackgroundSession,
    private val openOfficialFallback: () -> Unit,
) {
    fun open(part: WebChatProductionContentPart) {
        part.richCard?.let { card ->
            WebChatProductionRichCardViews.show(activity, card)
            return
        }
        if (part.type != "image") {
            openOfficialFallback()
            return
        }
        part.imageSource?.let { source ->
            ChatImageViewer.show(
                activity,
                ChatAttachment(
                    kind = "image",
                    displayName = part.label,
                    mimeType = part.mediaType ?: "image/jpeg",
                    localPath = source,
                    imageWidth = part.imageWidth,
                    imageHeight = part.imageHeight,
                ),
            )
            return
        }
        part.assetHandle?.let(session::retryImagePreview)
        Toast.makeText(activity, "正在准备图片预览…", Toast.LENGTH_SHORT).show()
    }
}

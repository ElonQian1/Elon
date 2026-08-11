package com.elon.app.chatgptweb

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context

internal class ChatGptMessageClipboard(context: Context) {
    private val clipboard = context.applicationContext
        .getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager

    fun copy(text: String): ChatGptClipboardMetadata {
        val clip = ClipData.newPlainText(CLIP_LABEL, text)
        clipboard.setPrimaryClip(clip)
        return ChatGptClipboardMetadata(
            hasPrimaryClip = clipboard.hasPrimaryClip(),
            itemCount = clip.itemCount,
            mimeTypes = buildSet {
                if (clip.description.hasMimeType("text/plain")) add("text/plain")
                if (clip.description.hasMimeType("text/html")) add("text/html")
            },
        )
    }

    private companion object {
        const val CLIP_LABEL = "ChatGPT message"
    }
}

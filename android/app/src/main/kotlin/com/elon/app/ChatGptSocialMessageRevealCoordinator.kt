package com.elon.app

import android.graphics.Rect
import android.view.View
import android.widget.LinearLayout
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation

internal class ChatGptSocialMessageRevealCoordinator(
    private val list: RecyclerView,
    private val providerId: () -> WebChatProviderId,
    private val transcript: WebChatProductionTranscript,
) {
    fun reveal(messageId: String, partIndex: Int?, target: String): Boolean {
        val nativeId = "${providerId().wireValue}:$messageId"
        val index = transcript.indexOfMessageId(nativeId)
        if (index < 0) return false
        val message = transcript.messageAt(index) ?: return false
        if (partIndex != null && partIndex !in message.webChatMessage?.contentParts.orEmpty().indices) {
            return false
        }
        val requiredAction = when (target) {
            "copy" -> WebChatMessageAction.COPY
            "regenerate" -> WebChatMessageAction.REGENERATE
            "actions" -> WebChatMessageAction.MORE
            else -> null
        }
        if (requiredAction != null && requiredAction !in message.webChatMessage?.actions.orEmpty()) {
            return false
        }
        list.scrollToPosition(index)
        revealTarget(index, messageId, partIndex, target, attempt = 0)
        return true
    }

    private fun revealTarget(
        index: Int,
        messageId: String,
        partIndex: Int?,
        target: String,
        attempt: Int,
    ) {
        list.postDelayed({
            val itemView = list.findViewHolderForAdapterPosition(index)?.itemView
            val targetView = itemView?.let { row ->
                partIndex?.let {
                    row.findViewById<LinearLayout>(R.id.webChatMessagePartList)?.getChildAt(it)
                } ?: when (target) {
                    "copy" -> row.findViewById<View>(R.id.webChatMessageCopy)
                    "regenerate" -> row.findViewById<View>(R.id.webChatMessageRegenerate)
                    "actions" -> row.findViewById<View>(R.id.webChatMessageMore)
                    else -> row
                }
            }
            if (itemView == null || targetView == null || targetView.visibility != View.VISIBLE) {
                retry(index, messageId, partIndex, target, attempt)
                return@postDelayed
            }
            itemView.contentDescription = "web-chat-message:${providerId().wireValue}:" +
                ChatGptNativeControlPresentation.stableContextId(messageId)
            val targetRect = Rect(0, 0, targetView.width.coerceAtLeast(1), targetView.height.coerceAtLeast(1))
            list.offsetDescendantRectToMyCoords(targetView, targetRect)
            val itemRect = Rect(0, 0, itemView.width.coerceAtLeast(1), itemView.height.coerceAtLeast(1))
            list.offsetDescendantRectToMyCoords(itemView, itemRect)
            targetRect.offset(-itemRect.left, -itemRect.top)
            list.requestChildRectangleOnScreen(itemView, targetRect, true)
            targetView.requestFocus()
            val visibleRect = Rect()
            val fullyVisible = targetView.getGlobalVisibleRect(visibleRect) &&
                visibleRect.width() >= targetView.width && visibleRect.height() >= targetView.height
            if (!fullyVisible) retry(index, messageId, partIndex, target, attempt)
        }, if (attempt == 0) 0L else REVEAL_RETRY_DELAY_MS)
    }

    private fun retry(
        index: Int,
        messageId: String,
        partIndex: Int?,
        target: String,
        attempt: Int,
    ) {
        if (attempt >= MAX_REVEAL_ATTEMPTS) return
        revealTarget(index, messageId, partIndex, target, attempt + 1)
    }

    private companion object {
        const val MAX_REVEAL_ATTEMPTS = 8
        const val REVEAL_RETRY_DELAY_MS = 80L
    }
}

package com.elon.app.socialquotes

import android.view.View
import android.widget.LinearLayout
import androidx.appcompat.app.AlertDialog
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.ChatMessage
import com.elon.app.R

internal object SocialQuoteRows {
    fun bind(root: View, message: ChatMessage, messages: List<ChatMessage>, enabled: Boolean) {
        val host = root.findViewById<LinearLayout>(R.id.messageQuote) ?: return
        host.removeAllViews()
        val quote = message.quote.takeIf { message.recalledAt.isNullOrBlank() }
        host.visibility = if (quote == null) View.GONE else View.VISIBLE
        quote ?: return
        val preview = SocialQuotePreview(root.context)
        host.addView(preview, LinearLayout.LayoutParams(-1, -2))
        preview.bind(quote, own = message.role == "user", open = if (!enabled) null else ({
            val position = if (quote.unavailable || quote.messageId.isBlank()) -1 else messages.indexOfFirst { it.id == quote.messageId }
            val recycler = generateSequence(root.parent) { it.parent }.filterIsInstance<RecyclerView>().firstOrNull()
            if (position >= 0 && recycler != null) recycler.smoothScrollToPosition(position)
            else AlertDialog.Builder(root.context).setTitle(quote.senderName.ifBlank { "引用消息" })
                .setMessage(SocialQuoteCodec.summary(quote)).setPositiveButton("关闭", null).show()
        }))
    }
}

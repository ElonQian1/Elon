package com.elon.app.chatgptweb

import android.text.TextUtils
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.R

internal class ChatGptNativeAttachmentController(
    private val scrollView: HorizontalScrollView,
    private val container: LinearLayout,
    private val onRemove: (String) -> Unit,
) {
    fun render(snapshot: ChatGptWebSnapshot) {
        container.removeAllViews()
        snapshot.attachments.forEach { attachment ->
            container.addView(createChip(attachment))
        }
        scrollView.visibility = if (snapshot.attachments.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun createChip(attachment: ChatGptWebAttachment): View {
        val context = container.context
        return LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = android.view.Gravity.CENTER_VERTICAL
            background = ContextCompat.getDrawable(context, R.drawable.bg_chatgpt_attachment_chip)
            setPadding(dp(10), 0, dp(4), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                dp(40),
            ).apply { marginEnd = dp(6) }
            addView(
                TextView(context).apply {
                    text = displayName(attachment)
                    maxWidth = dp(240)
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
                    textSize = 13f
                },
            )
            if (attachment.removable) {
                addView(
                    ImageButton(context).apply {
                        setImageResource(R.drawable.ic_chatgpt_attachment_remove)
                        setBackgroundColor(android.graphics.Color.TRANSPARENT)
                        contentDescription = context.getString(
                            R.string.chatgpt_native_attachment_remove,
                            attachment.name,
                        )
                        setPadding(dp(8), dp(8), dp(8), dp(8))
                        setOnClickListener { onRemove(attachment.id) }
                        layoutParams = LinearLayout.LayoutParams(dp(36), dp(36))
                    },
                )
            }
        }
    }

    private fun displayName(attachment: ChatGptWebAttachment): String = when (attachment.state) {
        "uploading" -> container.context.getString(
            R.string.chatgpt_native_attachment_uploading,
            attachment.name,
        )
        "error" -> container.context.getString(
            R.string.chatgpt_native_attachment_error,
            attachment.name,
        )
        else -> attachment.name
    }

    private fun dp(value: Int): Int = (value * container.resources.displayMetrics.density).toInt()
}

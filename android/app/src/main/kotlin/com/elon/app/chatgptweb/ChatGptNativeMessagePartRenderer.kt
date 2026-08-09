package com.elon.app.chatgptweb

import android.content.Context
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.R

internal class ChatGptNativeMessagePartRenderer(
    private val onOpenOfficial: () -> Unit,
) {
    fun render(container: LinearLayout, parts: List<ChatGptWebMessagePart>) {
        container.removeAllViews()
        parts.forEach { part -> container.addView(createRow(container, part)) }
        container.visibility = if (parts.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun createRow(container: LinearLayout, part: ChatGptWebMessagePart): TextView {
        val context = container.context
        return TextView(context).apply {
            text = context.getString(R.string.chatgpt_message_part_format, typeLabel(context, part.type), part.label)
            contentDescription = context.getString(R.string.chatgpt_message_part_open, text)
            gravity = Gravity.CENTER_VERTICAL
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            minHeight = dp(container, 44)
            setPadding(dp(container, 10), dp(container, 6), dp(container, 8), dp(container, 6))
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
            textSize = 13f
            background = ContextCompat.getDrawable(context, R.drawable.bg_chatgpt_attachment_chip)
            setCompoundDrawablesWithIntrinsicBounds(icon(part.type), 0, R.drawable.profile_icon_chevron, 0)
            compoundDrawablePadding = dp(container, 8)
            setOnClickListener { onOpenOfficial() }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ).apply { bottomMargin = dp(container, 4) }
        }
    }

    private fun typeLabel(context: Context, type: String): String = context.getString(
        when (type) {
            "image" -> R.string.chatgpt_message_part_image
            "file" -> R.string.chatgpt_message_part_file
            "citation" -> R.string.chatgpt_message_part_citation
            "artifact" -> R.string.chatgpt_message_part_artifact
            "audio" -> R.string.chatgpt_message_part_audio
            "video" -> R.string.chatgpt_message_part_video
            else -> R.string.chatgpt_message_part_content
        },
    )

    private fun icon(type: String): Int = when (type) {
        "image", "video" -> R.drawable.ic_attach_photos
        "audio" -> R.drawable.ic_input_voice
        "artifact" -> R.drawable.ic_project_documents_menu
        else -> R.drawable.ic_attach_files
    }

    private fun dp(container: LinearLayout, value: Int): Int =
        (value * container.resources.displayMetrics.density).toInt()
}

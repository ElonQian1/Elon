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
    fun render(container: LinearLayout, messageId: String, parts: List<ChatGptWebMessagePart>) {
        container.removeAllViews()
        parts.forEachIndexed { index, part ->
            container.addView(createRow(container, messageId, index, part))
        }
        container.visibility = if (parts.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun createRow(
        container: LinearLayout,
        messageId: String,
        index: Int,
        part: ChatGptWebMessagePart,
    ): TextView {
        val context = container.context
        return TextView(context).apply {
            text = displayText(context, part)
            contentDescription = ChatGptNativeControlPresentation.messagePartSelector(messageId, index, part.type)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
            tooltipText = context.getString(R.string.chatgpt_message_part_open, text)
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

    private fun displayText(context: Context, part: ChatGptWebMessagePart): String {
        val title = context.getString(
            R.string.chatgpt_message_part_format,
            typeLabel(context, part.type),
            part.label,
        )
        val detail = metadataSummary(context, part.metadata)
        return if (detail.isBlank()) title else "$title\n$detail"
    }

    private fun metadataSummary(context: Context, value: ChatGptWebMessagePartMetadata?): String {
        if (value == null) return ""
        return buildList {
            value.language?.let { add(context.getString(R.string.chatgpt_message_part_language, it)) }
            value.lineCount?.let { add(context.getString(R.string.chatgpt_message_part_lines, it)) }
            if (value.rowCount != null && value.columnCount != null) {
                add(context.getString(R.string.chatgpt_message_part_dimensions, value.rowCount, value.columnCount))
            }
            value.mediaType?.let { add(context.getString(R.string.chatgpt_message_part_media_type, it)) }
            value.targetHost?.let { add(context.getString(R.string.chatgpt_message_part_source, it)) }
        }.joinToString(" · ")
    }

    private fun typeLabel(context: Context, type: String): String = context.getString(
        when (type) {
            "image" -> R.string.chatgpt_message_part_image
            "file" -> R.string.chatgpt_message_part_file
            "citation" -> R.string.chatgpt_message_part_citation
            "code" -> R.string.chatgpt_message_part_code
            "table" -> R.string.chatgpt_message_part_table
            "artifact" -> R.string.chatgpt_message_part_artifact
            "audio" -> R.string.chatgpt_message_part_audio
            "video" -> R.string.chatgpt_message_part_video
            "math" -> R.string.chatgpt_message_part_math
            "chart" -> R.string.chatgpt_message_part_chart
            "map" -> R.string.chatgpt_message_part_map
            "interactive" -> R.string.chatgpt_message_part_interactive
            else -> R.string.chatgpt_message_part_content
        },
    )

    private fun icon(type: String): Int = when (type) {
        "image", "video", "chart", "map" -> R.drawable.ic_attach_photos
        "audio" -> R.drawable.ic_input_voice
        "artifact", "interactive" -> R.drawable.ic_project_documents_menu
        else -> R.drawable.ic_attach_files
    }

    private fun dp(container: LinearLayout, value: Int): Int =
        (value * container.resources.displayMetrics.density).toInt()
}

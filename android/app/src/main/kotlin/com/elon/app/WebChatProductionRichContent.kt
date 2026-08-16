package com.elon.app

import android.content.Context
import android.text.TextUtils
import android.text.method.LinkMovementMethod
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import io.noties.markwon.Markwon
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin
import java.util.WeakHashMap

internal object WebChatProductionRichContentBinder {
    private val renderers = WeakHashMap<Context, Markwon>()

    fun bindMessageText(view: TextView, message: ChatMessage): Boolean {
        val metadata = message.webChatMessage ?: return false
        if (!metadata.renderMarkdown || message.role != "friend") return false
        markwon(view.context).setMarkdown(view, message.content)
        view.movementMethod = LinkMovementMethod.getInstance()
        return true
    }

    fun bindParts(
        container: LinearLayout?,
        message: ChatMessage,
        onOpen: ((ChatMessage, WebChatProductionContentPart) -> Unit)?,
    ) {
        container ?: return
        container.removeAllViews()
        val metadata = message.webChatMessage
        val parts = metadata?.contentParts.orEmpty()
        parts.forEachIndexed { index, part ->
            container.addView(createPartRow(container, message, metadata!!, part, index, onOpen))
        }
        container.visibility = if (parts.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun createPartRow(
        container: LinearLayout,
        message: ChatMessage,
        metadata: WebChatProductionMessage,
        part: WebChatProductionContentPart,
        index: Int,
        onOpen: ((ChatMessage, WebChatProductionContentPart) -> Unit)?,
    ): TextView = TextView(container.context).apply {
        text = displayText(context, part)
        contentDescription = buildString {
            append("web-chat-message-part:")
            append(metadata.providerWireValue)
            append(':')
            append(ChatGptNativeControlPresentation.stableContextId(metadata.sourceMessageId))
            append(":$index:")
            append(ChatGptNativeControlPresentation.stableContextId(part.type))
        }
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
        gravity = Gravity.CENTER_VERTICAL
        maxLines = 2
        ellipsize = TextUtils.TruncateAt.END
        minHeight = dp(container, 48)
        setPadding(dp(container, 10), dp(container, 6), dp(container, 8), dp(container, 6))
        setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
        textSize = 13f
        background = ContextCompat.getDrawable(context, R.drawable.bg_chatgpt_attachment_chip)
        setCompoundDrawablesWithIntrinsicBounds(icon(part.type), 0, R.drawable.profile_icon_chevron, 0)
        compoundDrawablePadding = dp(container, 8)
        isClickable = onOpen != null
        isFocusable = onOpen != null
        tooltipText = context.getString(R.string.chatgpt_message_part_open, text)
        setOnClickListener(onOpen?.let { callback -> View.OnClickListener { callback(message, part) } })
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        ).apply { bottomMargin = dp(container, 4) }
    }

    private fun displayText(context: Context, part: WebChatProductionContentPart): String {
        val title = context.getString(
            R.string.chatgpt_message_part_format,
            typeLabel(context, part.type),
            part.label,
        )
        val details = buildList {
            part.language?.let { add(context.getString(R.string.chatgpt_message_part_language, it)) }
            part.lineCount?.let { add(context.getString(R.string.chatgpt_message_part_lines, it)) }
            if (part.rowCount != null && part.columnCount != null) {
                add(context.getString(R.string.chatgpt_message_part_dimensions, part.rowCount, part.columnCount))
            }
            part.mediaType?.let { add(context.getString(R.string.chatgpt_message_part_media_type, it)) }
            part.targetHost?.let { add(context.getString(R.string.chatgpt_message_part_source, it)) }
        }.joinToString(" · ")
        return if (details.isBlank()) title else "$title\n$details"
    }

    private fun typeLabel(context: Context, type: String): String = context.getString(
        when (type) {
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
        "video", "chart", "map" -> R.drawable.ic_attach_photos
        "audio" -> R.drawable.ic_input_voice
        "artifact", "interactive" -> R.drawable.ic_project_documents_menu
        else -> R.drawable.ic_attach_files
    }

    private fun markwon(context: Context): Markwon = synchronized(renderers) {
        val key = context.applicationContext
        renderers.getOrPut(key) {
            Markwon.builder(key)
                .usePlugin(StrikethroughPlugin.create())
                .usePlugin(TablePlugin.create(key))
                .build()
        }
    }

    private fun dp(container: LinearLayout, value: Int): Int =
        (value * container.resources.displayMetrics.density).toInt()
}

package com.elon.app

import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import org.json.JSONObject

internal object GroupAiReplyViews {
    fun bind(holder: ChatAdapter.VH, message: ChatMessage, action: ((ChatMessage, String) -> Unit)?) {
        val bubble = holder.bubble ?: return
        val parent = bubble.parent as? LinearLayout ?: return
        bubble.findViewWithTag<View>("group-ai-continue")?.let(bubble::removeView)
        parent.findViewWithTag<View>("group-ai-sources")?.let(parent::removeView)
        val metadata = message.groupAiReply?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return
        if (message.isRecalled() || action == null || metadata.optInt("schema") != 1) return
        val context = bubble.context
        fun label(value: String) = TextView(context).apply {
            text = value; textSize = 14f
            minHeight = (44 * resources.displayMetrics.density).toInt()
            gravity = android.view.Gravity.CENTER_VERTICAL
            setTextColor(ContextCompat.getColor(context, R.color.elon_link_primary))
        }
        if (metadata.optString("provider") == "chatgpt_web") {
            val footer = LinearLayout(context).apply { tag = "group-ai-continue"; orientation = LinearLayout.VERTICAL }
            footer.addView(label("使用 ChatGPT 继续讨论").apply {
                contentDescription = "group-ai-continue-discussion"
                setOnClickListener { action(message, "continue") }
            })
            if (metadata.optString("requester_id") == AuthManager.userId(context)) footer.addView(label("分享设置").apply {
                textSize = 12f; contentDescription = "group-ai-discussion-sharing"
                setOnClickListener { action(message, "sharing") }
            })
            bubble.addView(footer)
        }
        val count = metadata.optInt("source_count")
        parent.addView(LinearLayout(context).apply {
            tag = "group-ai-sources"; orientation = LinearLayout.VERTICAL
            val padding = (10 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding, padding, padding)
            background = android.graphics.drawable.GradientDrawable().apply {
                setColor(ContextCompat.getColor(context, R.color.elon_bg_app))
                setStroke(1, ContextCompat.getColor(context, R.color.elon_text_tertiary))
                cornerRadius = 6 * resources.displayMetrics.density
            }
            addView(label(if (count == 1) "引用的消息" else "群聊的聊天记录"))
            val previews = metadata.optJSONArray("previews")
            for (i in 0 until minOf(previews?.length() ?: 0, 3)) {
                val item = previews?.optJSONObject(i) ?: JSONObject()
                addView(label("${item.optString("sender_name")}：${item.optString("text").ifBlank { "[附件]" }}").apply {
                    textSize = 12f; minHeight = 0; maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                    setTextColor(ContextCompat.getColor(context, R.color.elon_text_secondary))
                })
            }
            addView(label("$count 条来源消息").apply { textSize = 12f })
            contentDescription = "group-ai-source-records-$count"
            isFocusable = true; setOnClickListener { action(message, "sources") }
        }, parent.indexOfChild(bubble) + 1, LinearLayout.LayoutParams(-1, -2).apply { topMargin = (8 * context.resources.displayMetrics.density).toInt() })
    }
}

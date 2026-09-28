package com.elon.app

import android.content.Context
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView

/** Compact account entry; complete usage and memory explanations belong in its details sheet. */
internal class GroupChatGptAccountView(context: Context, openDetails: () -> Unit) : LinearLayout(context) {
    private val title = label(14f, R.color.elon_text_primary)
    private val subtitle = label(12f, R.color.elon_text_secondary).apply { text = "账号与群聊记忆" }

    init {
        orientation = VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(48)
        setPadding(0, dp(4), dp(4), dp(4))
        isClickable = true
        isFocusable = true
        val value = android.util.TypedValue()
        context.theme.resolveAttribute(android.R.attr.selectableItemBackground, value, true)
        if (value.resourceId != 0) setBackgroundResource(value.resourceId)
        addView(title, LayoutParams(-1, -2))
        addView(subtitle, LayoutParams(-1, -2))
        tag = "group-chatgpt-account"
        setOnClickListener { openDetails() }
    }

    fun render(connected: Boolean) {
        title.text = if (connected) "我的 ChatGPT ›" else "接入 ChatGPT ›"
        contentDescription = "${title.text}；${subtitle.text}"
    }

    private fun label(size: Float, color: Int) = TextView(context).apply {
        textSize = size
        setTextColor(context.getColor(color))
    }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
}

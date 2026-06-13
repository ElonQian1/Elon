package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.content.Context
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView

internal fun bindChatSuggestionStatus(
    container: LinearLayout?,
    message: ChatMessage,
    onResolve: ((ChatMessage) -> Unit)?
) {
    if (container == null || message.suggestionStatus.isNullOrBlank()) return

    val context = container.context
    val updated = message.suggestionStatus == "updated"
    val actionable = !updated && message.canResolveSuggestion && onResolve != null
    val label = when {
        updated -> buildString {
            append("✓ 已更新")
            message.suggestionResolvedByName?.takeIf { it.isNotBlank() }?.let {
                append(" · ")
                append(it)
            }
        }
        actionable -> "□ 标记为已更新"
        else -> "□ 待开发者更新"
    }

    val statusView = TextView(context).apply {
        text = label
        textSize = 12f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(
            Color.parseColor(
                when {
                    updated -> "#58BE6A"
                    actionable -> "#101010"
                    else -> "#A8A8A8"
                }
            )
        )
        gravity = Gravity.CENTER
        setPadding(context.dp(10), context.dp(5), context.dp(10), context.dp(5))
        background = GradientDrawable().apply {
            cornerRadius = context.dp(12).toFloat()
            setColor(Color.parseColor(if (actionable) "#C8C8C8" else "#242424"))
        }
        isClickable = actionable
        isFocusable = actionable
        setOnClickListener(if (actionable) View.OnClickListener { onResolve?.invoke(message) } else null)
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            bottomMargin = context.dp(6)
        }
    }

    container.visibility = View.VISIBLE
    container.addView(statusView, 0)
}

private fun Context.dp(value: Int): Int =
    (value * resources.displayMetrics.density).toInt()

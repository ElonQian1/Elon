package com.elon.app

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.google.android.material.button.MaterialButton
import java.text.DateFormat
import java.util.Date

/** Production summary presentation; callbacks keep project and AI navigation with the activity. */
internal class AiWorkSummaryContent(private val context: Context,
    private val back: () -> Unit, private val selectDate: () -> Unit,
    private val openProject: (String) -> Unit,
    private val action: (GeneratedWorkSummaryItem, String) -> Unit) {
    private val uiColors = MobileColors(context)
    private fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
    private fun column() = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
    private fun label(value: String, size: Float = 16f, muted: Boolean = false) = TextView(context).apply {
        text = value; textSize = size; setTextColor(if (muted) uiColors.muted else uiColors.text)
        setPadding(0, dp(4), 0, dp(4))
    }
    private fun heading(value: String) = label(value, 20f).apply {
        setTypeface(typeface, Typeface.BOLD)
        androidx.core.view.ViewCompat.setAccessibilityHeading(this, true)
    }
    private fun button(value: String, primary: Boolean = false, click: () -> Unit) = MaterialButton(context).apply {
        text = value; textSize = 14f; isAllCaps = false
        minHeight = dp(48); minimumHeight = dp(48); minWidth = dp(48)
        insetTop = 0; insetBottom = 0
        setPadding(dp(16), dp(12), dp(16), dp(12))
        backgroundTintList = ColorStateList.valueOf(if (primary) uiColors.primary else uiColors.elevated)
        setTextColor(if (primary) uiColors.onPrimary else uiColors.text)
        setOnClickListener { click() }
    }
    private fun card() = column().apply {
        background = GradientDrawable().apply { setColor(uiColors.container); cornerRadius = dp(16).toFloat() }
        setPadding(dp(16), dp(12), dp(16), dp(12))
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) }
    }

    fun create(summary: GeneratedWorkSummary, day: String): ScrollView = ScrollView(context).apply {
        isFillViewport = true; setBackgroundColor(uiColors.surface)
        addView(column().apply {
            setPadding(dp(16), dp(8), dp(16), dp(24))
            addView(button("返回", click = back))
            addView(heading("AI 工作摘要"))
            addView(button("$day · 选择摘要日期", click = selectDate).apply { contentDescription = "选择摘要日期" })
            addView(label("根据 ${summary.projectCount} 个项目的状态整理", muted = true))
            addView(card().apply {
                addView(label("需要你关注 ${summary.attention.size} · 有新进展 ${summary.progress.size} · 待确认 ${summary.confirm.size}"))
            })
            addView(heading("需要你关注 · ${summary.attention.size}"))
            if (summary.attention.isEmpty()) addView(label("这一天没有需要处理的项目", muted = true))
            summary.attention.forEach { addView(attention(it)) }
            addView(fold("有新进展", summary.progress))
            addView(fold("待确认", summary.confirm))
        })
    }

    private fun attention(item: GeneratedWorkSummaryItem) = card().apply {
        if (item.highPriority) addView(label("高优先级", 14f).apply { setTextColor(uiColors.error) })
        addView(button(item.project) { openProject(item.project) })
        addView(heading(item.title))
        addView(label(item.reason, muted = true))
        addView(label("AI 建议", 14f).apply { setTextColor(uiColors.primary) })
        addView(label(item.suggestion))
        addView(button(item.secondaryAction) { action(item, item.secondaryAction) }, LinearLayout.LayoutParams(-1, -2))
        addView(button(item.primaryAction, item.highlightPrimary) { action(item, item.primaryAction) },
            LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(8) })
    }

    private fun fold(title: String, items: List<GeneratedWorkSummaryItem>) = column().apply {
        val content = column().apply {
            if (items.isEmpty()) addView(label("这一天没有$title", muted = true))
            items.forEach { item -> addView(card().apply {
                addView(button(item.project) { openProject(item.project) })
                addView(label(item.reason))
                addView(label(DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date(item.updatedAt)), 14f, true))
            }) }
        }
        lateinit var toggle: MaterialButton
        toggle = button("$title · ${items.size}  ▴") {
            val expanded = content.visibility != View.VISIBLE
            content.visibility = if (expanded) View.VISIBLE else View.GONE
            toggle.text = "$title · ${items.size}  ${if (expanded) "▴" else "▾"}"
            toggle.contentDescription = "${if (expanded) "收起" else "展开"}$title"
        }.apply { contentDescription = "收起$title" }
        addView(toggle, LinearLayout.LayoutParams(-1, -2))
        addView(content)
    }
}

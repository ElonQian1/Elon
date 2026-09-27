package com.elon.app

import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal data class HomeConversationCounts(
    val all: Int, val friends: Int, val projects: Int, val conversations: Int, val unread: Int
)

/** Content-sized header: filters scroll horizontally and the summary grows with system text. */
internal class HomeConversationHeaderView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private val colors by lazy { MobileColors(activity) }

    fun create(selected: HomeListFilterMode, counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit, onOpenSummary: () -> Unit): View =
        LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(colors.surface)
            addView(filters(selected, counts, onSelect))
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(16), dp(16), dp(16), dp(16))
                background = rounded(colors.container)
                isClickable = true; isFocusable = true
                foreground = selectableForeground()
                setOnClickListener { onOpenSummary() }
                contentDescription = "AI 工作摘要，${counts.projects}个项目，${counts.unread}条未读消息，查看详情"
                addView(label("AI 工作摘要", 20f, colors.text, true))
                addView(label("${counts.projects}个项目 · ${counts.unread}条未读消息", 16f, colors.muted).apply {
                    setPadding(0, dp(8), 0, dp(12))
                })
                addView(label("查看详情 ›", 14f, colors.primary, true))
            }, LinearLayout.LayoutParams(-1, -2).apply {
                marginStart = dp(16); marginEnd = dp(16); topMargin = dp(8)
            })
            addView(label("最近", 14f, colors.muted, true).apply {
                setPadding(dp(16), dp(24), dp(16), dp(8))
                androidx.core.view.ViewCompat.setAccessibilityHeading(this, true)
            })
        }

    private fun filters(selected: HomeListFilterMode, counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit): View = HorizontalScrollView(activity).apply {
        isHorizontalScrollBarEnabled = false
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(8), dp(16), dp(8))
            listOf(
                Triple(HomeListFilterMode.All, "全部", counts.all),
                Triple(HomeListFilterMode.Friends, "好友", counts.friends),
                Triple(HomeListFilterMode.Projects, "项目", counts.projects),
                Triple(HomeListFilterMode.Conversations, "群聊", counts.conversations)
            ).forEachIndexed { index, (mode, title, count) ->
                addView(label("$title ${count.coerceAtMost(99)}", 14f,
                    if (mode == selected) colors.onPrimaryContainer else colors.muted, mode == selected).apply {
                    minimumWidth = dp(48); minimumHeight = dp(48)
                    gravity = Gravity.CENTER
                    setPadding(dp(16), dp(12), dp(16), dp(12))
                    isSelected = mode == selected
                    background = rounded(if (isSelected) colors.primaryContainer else colors.surface)
                    isClickable = true; isFocusable = true
                    foreground = selectableForeground()
                    contentDescription = "$title，$count"
                    setOnClickListener { onSelect(mode) }
                }, LinearLayout.LayoutParams(-2, -2).apply { if (index > 0) marginStart = dp(8) })
            }
        })
    }

    private fun label(value: String, size: Float, ink: Int, bold: Boolean = false) = TextView(activity).apply {
        text = value; textSize = size; setTextColor(ink)
        typeface = Typeface.create("sans-serif", if (bold) Typeface.BOLD else Typeface.NORMAL)
    }

    private fun rounded(color: Int) = GradientDrawable().apply {
        setColor(color); cornerRadius = dp(16).toFloat()
    }
}

package com.elon.app

import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import android.content.Context

internal data class HomeConversationCounts(
    val all: Int, val friends: Int, val projects: Int, val conversations: Int, val unread: Int
)

/** Content-sized header: filters scroll horizontally and the summary grows with system text. */
internal class HomeConversationHeaderView(
    private val activity: Context,
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
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(16), dp(14), dp(16), dp(14))
                background = rounded(colors.container)
                isClickable = true; isFocusable = true
                foreground = selectableForeground()
                setOnClickListener { onOpenSummary() }
                contentDescription = "AI 工作摘要，${counts.projects}个项目，${counts.unread}条未读消息，查看详情"
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    addView(label("AI 工作摘要", 16f, colors.text, true))
                    addView(label("${counts.projects}个项目 · ${counts.unread}条未读消息", 13f, colors.muted).apply {
                        setPadding(0, dp(5), 0, 0)
                    })
                }, LinearLayout.LayoutParams(0, -2, 1f))
                addView(label("›", 24f, colors.muted).apply {
                    setPadding(dp(12), 0, 0, 0)
                    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                })
            }, LinearLayout.LayoutParams(-1, -2).apply {
                marginStart = dp(16); marginEnd = dp(16); topMargin = dp(8)
            })
            addView(label("最近", 14f, colors.muted, true).apply {
                setPadding(dp(16), dp(18), dp(16), dp(6))
                androidx.core.view.ViewCompat.setAccessibilityHeading(this, true)
            })
        }

    private fun filters(selected: HomeListFilterMode, counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit): View = HorizontalScrollView(activity).apply {
        isHorizontalScrollBarEnabled = false
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(4), dp(16), dp(4))
            listOf(
                Triple(HomeListFilterMode.All, "全部", counts.all),
                Triple(HomeListFilterMode.Friends, "好友", counts.friends),
                Triple(HomeListFilterMode.Projects, "项目", counts.projects),
                Triple(HomeListFilterMode.Conversations, "群聊", counts.conversations)
            ).forEachIndexed { index, (mode, title, count) ->
                addView(label("$title ${count.coerceAtMost(99)}", 14f,
                    if (mode == selected) colors.text else colors.muted, mode == selected).apply {
                    minimumWidth = dp(48); minimumHeight = dp(48)
                    gravity = Gravity.CENTER
                    setPadding(dp(14), dp(8), dp(14), dp(8))
                    isSelected = mode == selected
                    background = rounded(if (isSelected) colors.elevated else colors.surface)
                    isClickable = true; isFocusable = true
                    foreground = selectableForeground()
                    contentDescription = "$title，$count"
                    setOnClickListener { onSelect(mode) }
                }, LinearLayout.LayoutParams(-2, -2).apply { if (index > 0) marginStart = dp(4) })
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

package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal data class HomeConversationCounts(
    val all: Int,
    val friends: Int,
    val projects: Int,
    val conversations: Int,
    val unread: Int
)

internal class HomeConversationHeaderView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?
) {
    fun create(
        selected: HomeListFilterMode,
        counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit
    ): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        addView(createFilters(selected, counts, onSelect))
        addView(createSummaryCard(counts))
        addView(createRecentHeader())
    }

    private fun createFilters(
        selected: HomeListFilterMode,
        counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit
    ): View = HorizontalScrollView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(54))
        isHorizontalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(4), dp(16), dp(4))
            val items = listOf(
                HomeListFilterMode.All to ("全部" to counts.all),
                HomeListFilterMode.Friends to ("好友" to counts.friends),
                HomeListFilterMode.Projects to ("项目" to counts.projects),
                HomeListFilterMode.Conversations to ("对话" to counts.conversations),
                HomeListFilterMode.Unread to ("未读" to counts.unread)
            )
            items.forEachIndexed { index, (mode, label) ->
                addView(createFilterPill(label.first, label.second, mode == selected) { onSelect(mode) },
                    LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(36)).apply {
                        if (index > 0) marginStart = dp(10)
                    })
            }
        })
    }

    private fun createFilterPill(label: String, count: Int, selected: Boolean, onClick: () -> Unit): View =
        LinearLayout(activity).apply {
            minimumWidth = dp(if (label.length > 2) 66 else 62)
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(12), 0, dp(10), 0)
            background = pillBackground(selected)
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = label
                textSize = 15f
                setTextColor(Color.parseColor(if (selected) "#0B1118" else "#B3DDDBD5"))
            })
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(25), dp(22)).apply { marginStart = dp(5) }
                background = rounded("#9DB9D1", 11)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = count.coerceAtMost(99).toString()
                textSize = 12f
                typeface = Typeface.DEFAULT_BOLD
                setTextColor(Color.parseColor("#111820"))
            })
        }

    private fun createSummaryCard(counts: HomeConversationCounts): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(124)).apply {
            marginStart = dp(16); marginEnd = dp(16); topMargin = dp(8)
        }
        background = rounded("#171C22", 18)
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(20), 0, dp(18), 0)
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.ic_home_ai_avatar)
            scaleType = ImageView.ScaleType.FIT_CENTER
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "AI 工作摘要  Beta"
                textSize = 16f
                setTextColor(Color.parseColor("#F0F8F7F4"))
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "今天有${counts.projects}个项目需要你关注"
                textSize = 16f
                setTextColor(Color.parseColor("#F0F8F7F4"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(8) })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "${counts.unread}条重要消息 · ${counts.projects}个待处理事项"
                textSize = 13f
                setTextColor(Color.parseColor("#80BEBEBA"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(7) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(16) })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(86), dp(38))
            background = rounded("#353A42", 19)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "查看详情  ›"
            textSize = 13f
            setTextColor(Color.parseColor("#F0F8F7F4"))
        })
    }

    private fun createRecentHeader(): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(52))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(16), dp(5), dp(16), 0)
        addView(TextView(activity).apply {
            includeFontPadding = false; text = "最近"; textSize = 15f
            setTextColor(Color.parseColor("#B3DDDBD5"))
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        addView(TextView(activity).apply {
            includeFontPadding = false; text = "全部已读  ✓"; textSize = 14f
            setTextColor(Color.parseColor("#B3DDDBD5"))
        })
    }

    private fun pillBackground(selected: Boolean): GradientDrawable = GradientDrawable().apply {
        cornerRadius = dp(18).toFloat()
        if (selected) setColor(Color.parseColor("#9DB9D1")) else {
            setColor(Color.TRANSPARENT)
            setStroke(dp(1), Color.parseColor("#526C7884"))
        }
    }

    private fun rounded(color: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(color)); cornerRadius = dp(radius).toFloat()
    }
}

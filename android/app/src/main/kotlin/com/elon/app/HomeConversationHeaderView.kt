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
    private val regular = Typeface.create("sans-serif", Typeface.NORMAL)
    private val medium = Typeface.create("sans-serif-medium", Typeface.NORMAL)

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
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(52))
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
                HomeListFilterMode.Conversations to ("对话" to counts.conversations)
            )
            items.forEachIndexed { index, (mode, label) ->
                addView(createFilterPill(label.first, label.second, mode == selected) { onSelect(mode) },
                    LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(34)).apply {
                        if (index > 0) marginStart = dp(11)
                    })
            }
        })
    }

    private fun createFilterPill(label: String, count: Int, selected: Boolean, onClick: () -> Unit): View =
        LinearLayout(activity).apply {
            minimumWidth = dp(if (label.length > 2) 64 else 60)
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(11), 0, dp(9), 0)
            background = pillBackground(selected)
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = label
                textSize = 15f
                typeface = regular
                setTextColor(Color.parseColor(if (selected) "#0B1118" else "#B3DDDBD5"))
            })
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(24), dp(20)).apply { marginStart = dp(4) }
                background = rounded("#9CBAD5", 10)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = count.coerceAtMost(99).toString()
                textSize = 12f
                typeface = medium
                fontFeatureSettings = "tnum"
                setTextColor(Color.parseColor("#111820"))
            })
        }

    private fun createSummaryCard(counts: HomeConversationCounts): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(126)).apply {
            marginStart = dp(16); marginEnd = dp(16); topMargin = dp(7)
        }
        background = rounded("#171C22", 18)
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(20), 0, dp(18), 0)
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.ic_home_ai_avatar)
            scaleType = ImageView.ScaleType.FIT_CENTER
        }, LinearLayout.LayoutParams(dp(44), dp(44)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                orientation = LinearLayout.HORIZONTAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "AI 工作摘要"
                    textSize = 15f
                    typeface = regular
                    setTextColor(Color.parseColor("#F0F8F7F4"))
                })
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(19)).apply {
                        marginStart = dp(6)
                    }
                    background = rounded("#202B35", 4)
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    setPadding(dp(4), 0, dp(4), 0)
                    text = "Beta"
                    textSize = 10f
                    typeface = regular
                    setTextColor(Color.parseColor("#8FAEC5"))
                })
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "今天有${counts.projects}个项目需要你关注"
                textSize = 15f
                typeface = regular
                maxLines = 1
                setTextColor(Color.parseColor("#F0F8F7F4"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(9) })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "${counts.unread}条重要消息 · ${counts.projects}个待处理事项"
                textSize = 12f
                typeface = regular
                maxLines = 1
                setTextColor(Color.parseColor("#80BEBEBA"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(8) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(15) })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(82), dp(34)).apply { marginStart = dp(8) }
            background = rounded("#353A42", 17)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "查看详情  ❯"
            textSize = 12f
            typeface = regular
            setTextColor(Color.parseColor("#F0F8F7F4"))
        })
    }

    private fun createRecentHeader(): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(50))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(16), dp(5), dp(16), 0)
        addView(TextView(activity).apply {
            includeFontPadding = false; text = "最近"; textSize = 14f; typeface = regular
            setTextColor(Color.parseColor("#B3DDDBD5"))
        })
    }

    private fun pillBackground(selected: Boolean): GradientDrawable = GradientDrawable().apply {
        cornerRadius = dp(17).toFloat()
        if (selected) setColor(Color.parseColor("#9CBAD5")) else {
            setColor(Color.TRANSPARENT)
            setStroke(dp(1), Color.parseColor("#526C7884"))
        }
    }

    private fun rounded(color: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(color)); cornerRadius = dp(radius).toFloat()
    }
}

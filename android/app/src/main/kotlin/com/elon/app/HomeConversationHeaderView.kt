package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
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
        onSelect: (HomeListFilterMode) -> Unit,
        onOpenSummary: () -> Unit,
    ): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        addView(createFilters(selected, counts, onSelect))
        addView(createSummaryCard(counts, onOpenSummary))
        addView(createRecentHeader())
    }

    private fun createFilters(
        selected: HomeListFilterMode,
        counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit
    ): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(64))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(10), dp(3), dp(10), 0)
        val items = listOf(
            HomeListFilterMode.All to ("全部" to counts.all),
            HomeListFilterMode.Friends to ("好友" to counts.friends),
            HomeListFilterMode.Projects to ("项目" to counts.projects),
            HomeListFilterMode.Conversations to ("对话" to counts.conversations)
        )
        items.forEach { (mode, label) ->
            addView(
                createFilterTab(label.first, label.second, mode == selected) { onSelect(mode) },
                LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            )
        }
    }

    private fun createFilterTab(label: String, count: Int, selected: Boolean, onClick: () -> Unit): View =
        LinearLayout(activity).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            orientation = LinearLayout.VERTICAL
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            contentDescription = "$label，${count.coerceAtMost(99)}"
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER
                orientation = LinearLayout.HORIZONTAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = label
                    textSize = 17f
                    typeface = regular
                    setTextColor(Color.parseColor("#DDDBD5"))
                })
                addView(TextView(activity).apply {
                    minWidth = dp(28)
                    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(24)).apply {
                        marginStart = dp(4)
                    }
                    background = rounded("#9CBAD5", 12)
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    setPadding(dp(6), 0, dp(6), 0)
                    text = count.coerceAtMost(99).toString()
                    textSize = 13f
                    typeface = medium
                    fontFeatureSettings = "tnum"
                    setTextColor(Color.parseColor("#111820"))
                })
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(47)))
            addView(View(activity).apply {
                background = rounded(if (selected) "#9CBAD5" else "#00000000", 2)
            }, LinearLayout.LayoutParams(dp(42), dp(3)))
        }

    private fun createSummaryCard(counts: HomeConversationCounts, onOpenSummary: () -> Unit): View = LinearLayout(activity).apply {
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
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = "查看 AI 工作摘要详情"
            setOnClickListener { onOpenSummary() }
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

    private fun rounded(color: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(color)); cornerRadius = dp(radius).toFloat()
    }
}

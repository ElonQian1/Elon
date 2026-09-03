package com.elon.app

import android.graphics.Color
import android.graphics.Canvas
import android.graphics.ColorFilter
import android.graphics.Paint
import android.graphics.PixelFormat
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.HorizontalScrollView
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
        clipChildren = false
        clipToPadding = false
        setBackgroundColor(Color.parseColor("#131313"))
        addView(createFilters(selected, counts, onSelect))
        addView(createSummaryCard(counts, onOpenSummary))
        addView(createRecentHeader())
    }

    private fun createFilters(
        selected: HomeListFilterMode,
        counts: HomeConversationCounts,
        onSelect: (HomeListFilterMode) -> Unit
    ): View = HorizontalScrollView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(95))
        isHorizontalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        val items = listOf(
            HomeListFilterMode.All to ("全部" to counts.all),
            HomeListFilterMode.Friends to ("好友" to counts.friends),
            HomeListFilterMode.Projects to ("项目" to counts.projects),
            HomeListFilterMode.Conversations to ("对话" to counts.conversations)
        )
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(24), dp(16), dp(33))
            items.forEachIndexed { index, (mode, label) ->
                addView(
                    createFilterTab(label.first, label.second, mode == selected) { onSelect(mode) },
                    LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(38)).apply {
                        if (index > 0) marginStart = dp(16)
                    }
                )
            }
        })
    }

    private fun createFilterTab(label: String, count: Int, selected: Boolean, onClick: () -> Unit): View =
        LinearLayout(activity).apply {
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            contentDescription = "$label，${count.coerceAtMost(99)}"
            setPadding(dp(16), 0, dp(16), 0)
            background = roundedWithStroke(
                if (selected) "#2A2A2A" else "#00000000",
                24,
                if (selected) "#4DDBFCFF" else null
            )
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER
                orientation = LinearLayout.HORIZONTAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = label
                    textSize = 12f
                    typeface = regular
                    setTextColor(Color.parseColor(if (selected) "#DBFCFF" else "#B9CACB"))
                })
                addView(TextView(activity).apply {
                    minWidth = dp(20)
                    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(18)).apply {
                        marginStart = dp(8)
                    }
                    background = rounded(if (selected) "#335B6666" else "#353534", 9)
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    setPadding(dp(8), 0, dp(8), 0)
                    text = count.coerceAtMost(99).toString()
                    textSize = 12f
                    typeface = medium
                    fontFeatureSettings = "tnum"
                    setTextColor(Color.parseColor(if (selected) "#DBFCFF" else "#B9CACB"))
                })
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(32)))
        }

    private fun createSummaryCard(counts: HomeConversationCounts, onOpenSummary: () -> Unit): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(192)).apply {
            marginStart = dp(16); marginEnd = dp(16)
        }
        setLayerType(View.LAYER_TYPE_SOFTWARE, null)
        background = SummaryGlowDrawable(activity.resources.displayMetrics.density)
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(20), dp(20), dp(20), dp(20))
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.ic_home_ai_avatar)
            scaleType = ImageView.ScaleType.FIT_CENTER
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                orientation = LinearLayout.HORIZONTAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "AI 工作摘要"
                    textSize = 18f
                    typeface = Typeface.create("sans-serif", Typeface.BOLD)
                    setTextColor(Color.parseColor("#E5E2E1"))
                })
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(19)).apply {
                        marginStart = dp(6)
                    }
                    background = rounded("#4D563398", 10)
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    setPadding(dp(8), 0, dp(8), 0)
                    text = "Beta"
                    textSize = 12f
                    typeface = regular
                    setTextColor(Color.parseColor("#D2BBFF"))
                })
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "今天有${counts.projects}个项目需要你关注"
                textSize = 16f
                typeface = medium
                maxLines = 1
                setTextColor(Color.parseColor("#E5E2E1"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(12) })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "${counts.unread}条重要消息 · ${counts.projects}个待处理事项"
                textSize = 14f
                typeface = regular
                maxLines = 1
                setTextColor(Color.parseColor("#B9CACB"))
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(8) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(16) })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(92), dp(48)).apply { marginStart = dp(8) }
            background = rounded("#353534", 24)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "查看详情  ❯"
            textSize = 14f
            typeface = medium
            setTextColor(Color.parseColor("#E5E2E1"))
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = "查看 AI 工作摘要详情"
            setOnClickListener { onOpenSummary() }
        })
    }

    private fun createRecentHeader(): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(62)).apply { topMargin = dp(16) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(16), dp(16), dp(16), dp(16))
        addView(TextView(activity).apply {
            includeFontPadding = false; text = "最近"; textSize = 12f; typeface = medium
            letterSpacing = 0.05f
            setTextColor(Color.parseColor("#B9CACB"))
        })
    }

    private fun rounded(color: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(color)); cornerRadius = dp(radius).toFloat()
    }

    private fun roundedWithStroke(color: String, radius: Int, strokeColor: String?) =
        GradientDrawable().apply {
            setColor(Color.parseColor(color))
            cornerRadius = dp(radius).toFloat()
            strokeColor?.let { setStroke(dp(1), Color.parseColor(it)) }
        }

    private class SummaryGlowDrawable(density: Float) : Drawable() {
        private val radius = 12f * density
        private val inset = 1f * density
        private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.parseColor("#CC1A1A1A")
            setShadowLayer(30f * density, 0f, 0f, Color.parseColor("#2600F0FF"))
        }
        private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.STROKE
            strokeWidth = density
            color = Color.parseColor("#1AFFFFFF")
        }

        override fun draw(canvas: Canvas) {
            val rect = RectF(bounds).apply { inset(inset, inset) }
            canvas.drawRoundRect(rect, radius, radius, fill)
            canvas.drawRoundRect(rect, radius, radius, stroke)
        }

        override fun setAlpha(alpha: Int) { fill.alpha = alpha }
        override fun setColorFilter(colorFilter: ColorFilter?) { fill.colorFilter = colorFilter }
        @Deprecated("Deprecated in Android")
        override fun getOpacity(): Int = PixelFormat.TRANSLUCENT
    }
}

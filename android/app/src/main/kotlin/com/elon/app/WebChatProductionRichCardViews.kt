package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal object WebChatProductionRichCardViews {
    fun inline(
        container: LinearLayout,
        card: WebChatProductionRichCard,
        contentDescription: String,
        onClick: (() -> Unit)?,
    ): View = cardView(container.context, card, compact = true).apply {
        this.contentDescription = contentDescription
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
        isClickable = onClick != null
        isFocusable = onClick != null
        setOnClickListener(onClick?.let { action -> View.OnClickListener { action() } })
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        ).apply { bottomMargin = dp(container.context, 6) }
    }

    fun show(activity: AppCompatActivity, card: WebChatProductionRichCard) {
        if (activity.isFinishing || activity.isDestroyed) return
        val dialog = BottomSheetDialog(activity)
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(activity, 20), dp(activity, 12), dp(activity, 20), dp(activity, 24))
            background = roundedBackground(activity, R.color.elon_surface_card, 8)
            addView(View(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(activity, 36), dp(activity, 4)).apply {
                    gravity = Gravity.CENTER_HORIZONTAL
                    bottomMargin = dp(activity, 14)
                }
                background = roundedBackground(activity, R.color.elon_border_primary, 2)
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            })
            addView(detail(activity, card))
        }
        val scroll = ScrollView(activity).apply {
            isVerticalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            contentDescription = "web-chat-rich-card-detail:${card.kind.name.lowercase()}"
            addView(content)
        }
        dialog.setContentView(scroll)
        dialog.setOnShowListener {
            dialog.findViewById<FrameLayout>(com.google.android.material.R.id.design_bottom_sheet)?.let { sheet ->
                sheet.setBackgroundColor(Color.TRANSPARENT)
                BottomSheetBehavior.from(sheet).apply {
                    state = BottomSheetBehavior.STATE_EXPANDED
                    skipCollapsed = true
                }
            }
        }
        dialog.show()
    }

    internal fun detail(
        context: android.content.Context,
        card: WebChatProductionRichCard,
    ): View = cardView(context, card, compact = false).apply {
        contentDescription = "web-chat-rich-card-detail:${card.kind.name.lowercase()}"
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
    }

    private fun cardView(
        context: android.content.Context,
        card: WebChatProductionRichCard,
        compact: Boolean,
    ): LinearLayout = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(context, 14), dp(context, 13), dp(context, 14), dp(context, 14))
        background = roundedBackground(context, R.color.elon_surface_subtle, 8, R.color.elon_border_subtle)
        addView(textView(context, card.title, if (compact) 15f else 20f, bold = true).apply {
            maxLines = if (compact) 2 else 4
            ellipsize = TextUtils.TruncateAt.END
        })
        if (card.kind == WebChatProductionRichCard.Kind.FINANCE) {
            addFinanceHeader(context, card, compact)
        } else {
            card.description?.takeIf(String::isNotBlank)?.let { description ->
                addView(textView(context, description, 13f, R.color.elon_text_secondary).apply {
                    setPadding(0, dp(context, 7), 0, 0)
                    maxLines = if (compact) 3 else 8
                    ellipsize = TextUtils.TruncateAt.END
                })
            }
        }
        if (card.points.size >= 2 && card.series.isNotEmpty()) {
            addView(WebChatProductionLineChartView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(context, if (compact) 116 else 196),
                ).apply { topMargin = dp(context, 12) }
                setPadding(dp(context, 2), dp(context, 6), dp(context, 2), dp(context, 6))
                bind(card)
            })
        }
        if (card.kind == WebChatProductionRichCard.Kind.CHART) addSeriesLegend(context, card)
        if (card.metrics.isNotEmpty()) addMetrics(context, card.metrics, compact)
    }

    private fun LinearLayout.addFinanceHeader(
        context: android.content.Context,
        card: WebChatProductionRichCard,
        compact: Boolean,
    ) {
        val headline = listOfNotNull(card.symbol, card.primaryValue).joinToString("  ")
        if (headline.isNotBlank()) {
            addView(textView(context, headline, if (compact) 22f else 28f, bold = true).apply {
                setPadding(0, dp(context, 8), 0, 0)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
        }
        card.secondaryValue?.takeIf(String::isNotBlank)?.let { secondary ->
            addView(textView(context, secondary, 13f, trendColor(card.trend)).apply {
                setPadding(0, dp(context, 4), 0, 0)
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
            })
        }
        card.periods.takeIf(List<WebChatProductionRichCard.Period>::isNotEmpty)?.let { periods ->
            addView(textView(
                context,
                periods.joinToString("   ") { if (it.selected) "● ${it.label}" else it.label },
                12f,
                R.color.elon_text_secondary,
            ).apply {
                setPadding(0, dp(context, 10), 0, 0)
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
            })
        }
    }

    private fun LinearLayout.addSeriesLegend(
        context: android.content.Context,
        card: WebChatProductionRichCard,
    ) {
        val row = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = dp(context, 7) }
        }
        card.series.take(4).forEachIndexed { index, series ->
            row.addView(textView(context, "● ${series.label}", 11f, seriesColor(index)).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
        }
        addView(row)
    }

    private fun LinearLayout.addMetrics(
        context: android.content.Context,
        metrics: List<WebChatProductionRichCard.Metric>,
        compact: Boolean,
    ) {
        metrics.take(if (compact) 4 else 16).chunked(2).forEachIndexed { rowIndex, pair ->
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ).apply { topMargin = dp(context, if (rowIndex == 0) 12 else 7) }
                pair.forEach { metric ->
                    addView(LinearLayout(context).apply {
                        orientation = LinearLayout.VERTICAL
                        layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                        addView(textView(context, metric.label, 11f, R.color.elon_text_tertiary).apply {
                            maxLines = 1
                            ellipsize = TextUtils.TruncateAt.END
                        })
                        addView(textView(context, metric.value, 13f, bold = true).apply {
                            setPadding(0, dp(context, 3), dp(context, 8), 0)
                            maxLines = 2
                            ellipsize = TextUtils.TruncateAt.END
                        })
                    })
                }
                if (pair.size == 1) addView(View(context).apply {
                    layoutParams = LinearLayout.LayoutParams(0, 1, 1f)
                })
            })
        }
    }

    private fun textView(
        context: android.content.Context,
        value: String,
        size: Float,
        color: Int = R.color.elon_text_primary,
        bold: Boolean = false,
    ) = TextView(context).apply {
        text = value
        textSize = size
        includeFontPadding = false
        setTextColor(ContextCompat.getColor(context, color))
        if (bold) setTypeface(typeface, Typeface.BOLD)
    }

    private fun trendColor(trend: WebChatProductionRichCard.Trend?): Int = when (trend) {
        WebChatProductionRichCard.Trend.POSITIVE -> R.color.elon_status_success
        WebChatProductionRichCard.Trend.NEGATIVE -> R.color.elon_status_danger
        else -> R.color.elon_text_secondary
    }

    private fun seriesColor(index: Int): Int = when (index) {
        1 -> R.color.elon_status_success
        2 -> R.color.elon_status_project
        3 -> R.color.elon_status_danger
        else -> R.color.elon_accent_primary
    }

    private fun roundedBackground(
        context: android.content.Context,
        color: Int,
        radiusDp: Int,
        strokeColor: Int? = null,
    ) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(context, radiusDp).toFloat()
        setColor(ContextCompat.getColor(context, color))
        strokeColor?.let { setStroke(dp(context, 1), ContextCompat.getColor(context, it)) }
    }

    private fun dp(context: android.content.Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}

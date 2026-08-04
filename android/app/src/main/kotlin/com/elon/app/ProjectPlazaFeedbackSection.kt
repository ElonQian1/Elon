package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class ProjectPlazaFeedbackSection(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    fun buildLoading(): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(SIDE_MARGIN_DP), dp(20), dp(SIDE_MARGIN_DP), dp(28))
        contentDescription = "正在加载项目"
        repeat(3) { index ->
            addView(skeletonRow(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(SKELETON_ROW_HEIGHT_DP)
            ).apply {
                if (index > 0) topMargin = dp(8)
            })
        }
    }

    fun buildEmpty(actionLabel: String, onAction: () -> Unit): View = messagePanel(
        title = "没有找到匹配项目",
        message = "可以清除搜索与筛选条件，再看看项目广场的全部内容。",
        toneColor = COLOR_TEXT_TERTIARY,
        actionLabel = actionLabel,
        onAction = onAction
    )

    fun buildError(message: String, onRetry: () -> Unit): View = messagePanel(
        title = "项目暂时没有加载出来",
        message = message.ifBlank { "请检查网络连接后重试。" },
        toneColor = COLOR_DANGER,
        actionLabel = "重新加载",
        onAction = onRetry
    )

    private fun skeletonRow(): View = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        background = rect(COLOR_SURFACE, 18)
        setPadding(dp(16), dp(14), dp(16), dp(14))
        addView(View(activity).apply {
            background = rect(COLOR_SKELETON_STRONG, 12)
            contentDescription = null
        }, LinearLayout.LayoutParams(dp(60), dp(60)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(skeletonLine(COLOR_SKELETON_STRONG), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(12)
            ))
            addView(skeletonLine(COLOR_SKELETON_SOFT), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(10)
            ).apply { topMargin = dp(10) })
            addView(skeletonLine(COLOR_SKELETON_SOFT), LinearLayout.LayoutParams(
                dp(132),
                dp(10)
            ).apply { topMargin = dp(8) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
            marginStart = dp(16)
        })
    }

    private fun skeletonLine(color: String): View = View(activity).apply {
        background = rect(color, 5)
        contentDescription = null
    }

    private fun messagePanel(
        title: String,
        message: String,
        toneColor: String,
        actionLabel: String,
        onAction: () -> Unit
    ): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        background = rect(COLOR_SURFACE, 20)
        setPadding(dp(20), dp(24), dp(20), dp(20))
        addView(View(activity).apply {
            background = rect(toneColor, 4)
            contentDescription = null
        }, LinearLayout.LayoutParams(dp(8), dp(8)))
        addView(TextView(activity).apply {
            text = title
            includeFontPadding = false
            gravity = Gravity.CENTER
            setTextColor(Color.WHITE)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
            typeface = Typeface.DEFAULT_BOLD
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(14) })
        addView(TextView(activity).apply {
            text = message
            includeFontPadding = false
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
            setLineSpacing(0f, 1.12f)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(8) })
        addView(TextView(activity).apply {
            text = actionLabel
            gravity = Gravity.CENTER
            includeFontPadding = false
            background = rect(Color.WHITE, ACTION_HEIGHT_DP / 2)
            foreground = selectableForeground()
            isClickable = true
            setTextColor(Color.BLACK)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            typeface = Typeface.DEFAULT_BOLD
            setOnClickListener { onAction() }
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(ACTION_HEIGHT_DP)
        ).apply { topMargin = dp(20) })
    }.also { panel ->
        panel.layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(SIDE_MARGIN_DP)
            marginEnd = dp(SIDE_MARGIN_DP)
            topMargin = dp(24)
            bottomMargin = dp(24)
        }
    }

    private fun rect(color: String, radiusDp: Int): GradientDrawable =
        rect(Color.parseColor(color), radiusDp)

    private fun rect(color: Int, radiusDp: Int): GradientDrawable = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        setColor(color)
        cornerRadius = dp(radiusDp).toFloat()
    }

    private companion object {
        const val COLOR_SURFACE = "#1A1A1A"
        const val COLOR_SKELETON_STRONG = "#272727"
        const val COLOR_SKELETON_SOFT = "#6D6E6F"
        const val COLOR_TEXT_SECONDARY = "#B8B8B8"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val COLOR_DANGER = "#E62129"
        const val SIDE_MARGIN_DP = 20
        const val SKELETON_ROW_HEIGHT_DP = 88
        const val ACTION_HEIGHT_DP = 48
    }
}

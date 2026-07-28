package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import java.time.LocalDate
import java.time.format.TextStyle
import java.util.Locale

internal fun createSocialSidebarDateStrip(
    context: Context,
    selectedDate: LocalDate,
    onDateSelected: (LocalDate) -> Unit,
    dp: (Int) -> Int,
    selectableForeground: () -> Drawable?
): LinearLayout = LinearLayout(context).apply {
    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(84)).apply {
        topMargin = dp(4)
        bottomMargin = dp(8)
    }
    gravity = Gravity.CENTER
    orientation = LinearLayout.HORIZONTAL
    (-3L..3L).forEach { offset ->
        val date = selectedDate.plusDays(offset)
        addView(
            socialSidebarDateCell(
                context = context,
                date = date,
                selected = offset == 0L,
                offset = offset,
                onDateSelected = onDateSelected,
                dp = dp,
                selectableForeground = selectableForeground
            ),
            LinearLayout.LayoutParams(0, dp(76), 1f)
        )
    }
}

private fun socialSidebarDateCell(
    context: Context,
    date: LocalDate,
    selected: Boolean,
    offset: Long,
    onDateSelected: (LocalDate) -> Unit,
    dp: (Int) -> Int,
    selectableForeground: () -> Drawable?
): FrameLayout = FrameLayout(context).apply {
    isClickable = true
    foreground = selectableForeground()
    contentDescription = "${date.monthValue}月${date.dayOfMonth}日"
    if (selected) addView(ImageView(context).apply {
        setImageResource(R.drawable.social_sidebar_date_pill)
        scaleType = ImageView.ScaleType.FIT_XY
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }, FrameLayout.LayoutParams(dp(35), dp(67)).apply { gravity = Gravity.CENTER })
    addView(LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        val contentGravity = socialSidebarDateContentGravity(offset)
        val contentOffsetPx = dp(socialSidebarDateContentOffsetDp(offset)).toFloat()
        addView(
            socialSidebarDateLabel(context, date.dayOfMonth.toString(), 14f, selected).apply {
                translationX = contentOffsetPx
            },
            socialSidebarDateLabelParams(contentGravity)
        )
        addView(
            socialSidebarDateLabel(
                context,
                date.dayOfWeek.getDisplayName(TextStyle.SHORT, Locale.ENGLISH),
                14f,
                selected
            ).apply {
                setPadding(0, dp(6), 0, 0)
                translationX = contentOffsetPx
            },
            socialSidebarDateLabelParams(contentGravity)
        )
    }, FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.MATCH_PARENT
    ))
    setOnClickListener { onDateSelected(date) }
}

private fun socialSidebarDateContentGravity(offset: Long): Int = when (offset) {
    -3L -> Gravity.START
    3L -> Gravity.END
    else -> Gravity.CENTER_HORIZONTAL
}

private fun socialSidebarDateContentOffsetDp(offset: Long): Int = when (offset) {
    -2L, -1L, 1L, 2L -> offset.toInt() * 3
    else -> 0
}

private fun socialSidebarDateLabelParams(contentGravity: Int) =
    LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
    ).apply {
        gravity = contentGravity
    }

private fun socialSidebarDateLabel(
    context: Context,
    value: String,
    size: Float,
    selected: Boolean
) =
    TextView(context).apply {
        includeFontPadding = false
        gravity = Gravity.CENTER
        text = value
        setSingleLine(true)
        textSize = size
        setTextColor(Color.parseColor(if (selected) "#464646" else "#D9D9D9"))
    }

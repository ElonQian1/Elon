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
    layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(112)).apply {
        topMargin = dp(8)
        bottomMargin = dp(12)
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
                onDateSelected = onDateSelected,
                dp = dp,
                selectableForeground = selectableForeground
            ),
            LinearLayout.LayoutParams(0, dp(102), 1f)
        )
    }
}

private fun socialSidebarDateCell(
    context: Context,
    date: LocalDate,
    selected: Boolean,
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
    }, FrameLayout.LayoutParams(dp(47), dp(90)).apply { gravity = Gravity.CENTER })
    addView(LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER
        addView(socialSidebarDateLabel(context, date.dayOfMonth.toString(), 18f, selected))
        addView(
            socialSidebarDateLabel(
                context,
                date.dayOfWeek.getDisplayName(TextStyle.SHORT, Locale.ENGLISH),
                16f,
                selected
            ).apply { setPadding(0, dp(10), 0, 0) }
        )
    }, FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.MATCH_PARENT
    ))
    setOnClickListener { onDateSelected(date) }
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
        textSize = size
        setTextColor(Color.parseColor(if (selected) "#464646" else "#D9D9D9"))
    }

package com.elon.app

import android.content.Context
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import java.time.LocalDate
import java.time.format.TextStyle
import java.util.Locale

/** Content-sized dates remain reachable in narrow sidebars and at large font scales. */
internal fun createSocialSidebarDateStrip(
    context: Context,
    selectedDate: LocalDate,
    onDateSelected: (LocalDate) -> Unit,
    dp: (Int) -> Int,
    selectableForeground: () -> Drawable?,
    dateContentDescription: (LocalDate) -> String = { date ->
        "${date.monthValue}月${date.dayOfMonth}日"
    },
): HorizontalScrollView = HorizontalScrollView(context).apply {
    isHorizontalScrollBarEnabled = false
    layoutParams = LinearLayout.LayoutParams(-1, -2).apply {
        topMargin = dp(4)
        bottomMargin = dp(8)
    }
    val uiColors = MobileColors(context)
    val dates = LinearLayout(context).apply { orientation = LinearLayout.HORIZONTAL }
    var selectedCell: TextView? = null
    (-3L..3L).forEach { offset ->
        val date = selectedDate.plusDays(offset)
        val selected = offset == 0L
        val cell = TextView(context).apply {
            text = "${date.dayOfMonth}\n${date.dayOfWeek.getDisplayName(TextStyle.SHORT, Locale.getDefault())}"
            textSize = 14f
            gravity = Gravity.CENTER
            minWidth = dp(48)
            minHeight = dp(64)
            setPadding(dp(8), dp(8), dp(8), dp(8))
            setTextColor(if (selected) uiColors.onPrimaryContainer else uiColors.text)
            isSelected = selected
            contentDescription = dateContentDescription(date)
            background = GradientDrawable().apply {
                cornerRadius = dp(12).toFloat()
                setColor(if (selected) uiColors.primaryContainer else uiColors.surface)
            }
            foreground = selectableForeground()
            setOnClickListener { onDateSelected(date) }
        }
        dates.addView(cell, LinearLayout.LayoutParams(-2, -2).apply { marginEnd = dp(4) })
        if (selected) selectedCell = cell
    }
    addView(dates)
    post { selectedCell?.let { scrollTo((it.left - (width - it.width) / 2).coerceAtLeast(0), 0) } }
}

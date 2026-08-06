package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.widget.LinearLayout
import androidx.core.content.ContextCompat

internal fun panelBackground(color: String): GradientDrawable {
    return GradientDrawable().apply {
        setColor(Color.parseColor(color))
        cornerRadius = 0f
    }
}

internal fun projectSpaceDivider(context: Context, dp: (Int) -> Int): View {
    return View(context).apply {
        setBackgroundColor(ContextCompat.getColor(context, R.color.elon_divider_card))
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            1
        ).apply {
            marginStart = dp(20)
            marginEnd = dp(20)
        }
    }
}

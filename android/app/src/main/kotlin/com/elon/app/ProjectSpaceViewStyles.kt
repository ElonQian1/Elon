package com.elon.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable

internal fun panelBackground(color: String): GradientDrawable {
    return GradientDrawable().apply {
        setColor(Color.parseColor(color))
        cornerRadius = 0f
    }
}

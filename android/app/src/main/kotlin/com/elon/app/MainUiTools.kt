package com.elon.app

import android.graphics.drawable.Drawable
import android.util.TypedValue
import androidx.appcompat.app.AppCompatActivity

internal class MainUiTools(private val activity: AppCompatActivity) {
    fun selectableForeground(): Drawable? = runCatching {
        val outValue = TypedValue()
        activity.theme.resolveAttribute(android.R.attr.selectableItemBackground, outValue, true)
        activity.getDrawable(outValue.resourceId)
    }.getOrNull()

    fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density).toInt()
    }

    fun shareActions(): MainShareActions {
        return MainShareActions(activity, ::dp)
    }
}

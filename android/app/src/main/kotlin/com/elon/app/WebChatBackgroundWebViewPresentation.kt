package com.elon.app

import android.graphics.Color
import android.view.View
import android.webkit.WebView

internal fun WebView.configureWebChatBackgroundSurface() {
    setBackgroundColor(Color.TRANSPARENT)
    visibility = View.INVISIBLE
    isClickable = false
    isFocusable = false
    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
}

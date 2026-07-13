package com.elon.app

import android.graphics.Color
import android.os.Build
import android.view.View
import android.view.Window
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import com.elon.app.databinding.ActivityMainBinding

internal fun shouldDrawChatBehindNavigationBar(binding: ActivityMainBinding): Boolean {
    return binding.chatPage.visibility == View.VISIBLE &&
        (binding.inputLayout.visibility == View.VISIBLE ||
            binding.chatSelectionBar.visibility == View.VISIBLE)
}

internal fun applyMainSystemBarChrome(activity: AppCompatActivity, binding: ActivityMainBinding?) {
    val window = activity.window
    window.statusBarColor = ContextCompat.getColor(activity, R.color.elon_bg_app)
    val navigationColor = if (binding?.projectSpaceAiMenu?.visibility == View.VISIBLE) {
        R.color.elon_store_detail_bg
    } else {
        R.color.elon_bg_app
    }
    var flags = window.decorView.systemUiVisibility
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
        flags = flags and View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR.inv()
    }
    window.decorView.systemUiVisibility = flags
    applyMainNavigationBarChrome(
        window = window,
        root = binding?.root,
        drawChatBehindNavigationBar = binding?.let(::shouldDrawChatBehindNavigationBar) == true,
        opaqueColor = ContextCompat.getColor(activity, navigationColor)
    )
}

internal fun ActivityMainBinding.scheduleNavigationBarChrome(
    activity: AppCompatActivity,
    colorRes: Int,
    drawChatBehindNavigationBar: Boolean = shouldDrawChatBehindNavigationBar(this)
) {
    val applyColor = {
        applyMainNavigationBarChrome(
            window = activity.window,
            root = root,
            drawChatBehindNavigationBar = drawChatBehindNavigationBar,
            opaqueColor = activity.getColor(colorRes)
        )
    }
    applyColor()
    root.post { applyColor() }
}

internal fun applyMainNavigationBarChrome(
    window: Window,
    root: View?,
    drawChatBehindNavigationBar: Boolean,
    opaqueColor: Int
) {
    window.navigationBarColor = if (drawChatBehindNavigationBar) Color.TRANSPARENT else opaqueColor
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        window.isNavigationBarContrastEnforced = false
    }

    var flags = window.decorView.systemUiVisibility
    flags = if (drawChatBehindNavigationBar) {
        flags or View.SYSTEM_UI_FLAG_LAYOUT_STABLE or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    } else {
        flags and View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION.inv()
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        flags = flags and View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR.inv()
    }
    window.decorView.systemUiVisibility = flags
    root?.let(ViewCompat::requestApplyInsets)
}

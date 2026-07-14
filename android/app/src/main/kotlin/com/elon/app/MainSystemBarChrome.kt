package com.elon.app

import android.graphics.Color
import android.os.Build
import android.view.View
import android.view.ViewGroup
import android.view.Window
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
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

    window.decorView.systemUiVisibility = resolveMainSystemUiVisibility(
        currentFlags = window.decorView.systemUiVisibility,
        drawChatBehindNavigationBar = drawChatBehindNavigationBar,
        sdkInt = Build.VERSION.SDK_INT
    )
    root?.let(ViewCompat::requestApplyInsets)
}

internal fun resolveMainSystemUiVisibility(
    currentFlags: Int,
    drawChatBehindNavigationBar: Boolean,
    sdkInt: Int
): Int {
    var flags = currentFlags and View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN.inv()
    flags = flags and View.SYSTEM_UI_FLAG_FULLSCREEN.inv()
    flags = if (drawChatBehindNavigationBar) {
        flags or View.SYSTEM_UI_FLAG_LAYOUT_STABLE or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    } else {
        flags and View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION.inv()
    }
    if (sdkInt >= Build.VERSION_CODES.O) {
        flags = flags and View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR.inv()
    }
    return flags
}

internal fun ActivityMainBinding.restoreChatToolbar(backgroundColor: Int) {
    toolbar.visibility = View.VISIBLE
    toolbar.alpha = 1f
    toolbar.translationY = 0f
    toolbar.setBackgroundColor(backgroundColor)
    topTitleText.visibility = View.VISIBLE
    ViewCompat.requestApplyInsets(root)
}

internal fun applyMainToolbarStatusBarInset(
    binding: ActivityMainBinding,
    insets: WindowInsetsCompat
) {
    val params = binding.toolbar.layoutParams as? ViewGroup.MarginLayoutParams ?: return
    val statusBarResourceId = binding.root.resources.getIdentifier(
        "status_bar_height",
        "dimen",
        "android"
    )
    val statusBarResourceHeight = if (statusBarResourceId != 0) {
        binding.root.resources.getDimensionPixelSize(statusBarResourceId)
    } else {
        0
    }
    val statusBarTop = maxOf(
        insets.getInsets(WindowInsetsCompat.Type.statusBars()).top,
        statusBarResourceHeight
    )
    val location = IntArray(2)
    binding.toolbar.getLocationInWindow(location)
    val toolbarTopWithoutAppliedMargin = location[1] - params.topMargin
    val targetTopMargin = (statusBarTop - toolbarTopWithoutAppliedMargin).coerceAtLeast(0)
    if (params.topMargin == targetTopMargin) return

    params.topMargin = targetTopMargin
    binding.toolbar.layoutParams = params
    binding.toolbar.post { ViewCompat.requestApplyInsets(binding.root) }
}

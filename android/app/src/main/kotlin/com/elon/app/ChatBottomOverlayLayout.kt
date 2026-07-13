package com.elon.app

import android.widget.FrameLayout
import com.elon.app.databinding.ActivityMainBinding

internal fun syncChatSideMenuHandleBottomMargin(
    binding: ActivityMainBinding,
    bottomOverlayHeight: Int
) {
    val handle = binding.chatSideMenuHandleButton
    val params = handle.layoutParams as? FrameLayout.LayoutParams ?: return
    val gap = (handle.resources.displayMetrics.density * HANDLE_OVERLAY_GAP_DP + 0.5f).toInt()
    val targetMargin = bottomOverlayHeight + gap
    if (params.bottomMargin == targetMargin) return
    params.bottomMargin = targetMargin
    handle.layoutParams = params
}

private const val HANDLE_OVERLAY_GAP_DP = 12

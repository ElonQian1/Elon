package com.elon.app

import android.view.ViewGroup
import androidx.core.view.WindowInsetsCompat
import com.elon.app.databinding.ActivityMainBinding

/** Keeps the shared home/chat toolbar aligned below the physical status bar. */
internal class MainToolbarStatusBarInsetController(
    private val binding: ActivityMainBinding
) {
    private val rootWindowLocation = IntArray(2)
    private var statusBarInsetTop = 0

    fun install() {
        binding.root.viewTreeObserver.addOnPreDrawListener {
            val params = binding.toolbar.layoutParams as? ViewGroup.MarginLayoutParams
                ?: return@addOnPreDrawListener true
            binding.root.getLocationInWindow(rootWindowLocation)
            val targetTopMargin = resolveMainToolbarTopMargin(
                statusBarInsetTop = statusBarInsetTop,
                rootTopInWindow = rootWindowLocation[1]
            )
            if (params.topMargin == targetTopMargin) {
                return@addOnPreDrawListener true
            }
            params.topMargin = targetTopMargin
            binding.toolbar.layoutParams = params
            false
        }
    }

    fun update(insets: WindowInsetsCompat) {
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
        statusBarInsetTop = maxOf(
            insets.getInsets(WindowInsetsCompat.Type.statusBars()).top,
            statusBarResourceHeight
        )
    }
}

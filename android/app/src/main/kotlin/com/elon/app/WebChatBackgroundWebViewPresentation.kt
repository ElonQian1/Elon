package com.elon.app

import android.graphics.Color
import android.view.View
import android.webkit.WebView

internal fun WebView.configureWebChatBackgroundSurface() {
    setBackgroundColor(Color.TRANSPARENT)
    isClickable = false
    isFocusable = false
    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
    endWebChatBackgroundInteraction()
}

internal fun WebView.beginWebChatBackgroundInteraction() {
    alpha = 0f
    visibility = View.VISIBLE
}

internal fun WebView.endWebChatBackgroundInteraction() {
    visibility = View.INVISIBLE
    alpha = 1f
}

internal fun WebView.showWebChatSkinSurface() {
    alpha = 1f
    visibility = View.VISIBLE
    isClickable = true
    isFocusable = true
    isFocusableInTouchMode = true
    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
}

internal fun WebView.showWebChatBackgroundSurface() {
    isClickable = false
    isFocusable = false
    isFocusableInTouchMode = false
    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
    endWebChatBackgroundInteraction()
}

internal class WebChatBackgroundInteractionLease(
    private val webView: () -> WebView?,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
) {
    private var generation = 0L
    private var leasedView: WebView? = null
    private var scheduledRelease: Runnable? = null

    fun run(action: () -> Unit): Boolean {
        val view = webView() ?: return false
        release()
        generation += 1
        val expectedGeneration = generation
        leasedView = view
        view.beginWebChatBackgroundInteraction()
        view.postOnAnimation {
            if (generation != expectedGeneration || leasedView !== view) return@postOnAnimation
            action()
            lateinit var release: Runnable
            release = Runnable {
                if (scheduledRelease !== release || generation != expectedGeneration) return@Runnable
                scheduledRelease = null
                leasedView = null
                view.endWebChatBackgroundInteraction()
            }
            scheduledRelease = release
            schedule(release, MAX_LEASE_MS)
        }
        return true
    }

    fun release() {
        generation += 1
        scheduledRelease?.let(cancel)
        scheduledRelease = null
        leasedView?.endWebChatBackgroundInteraction()
        leasedView = null
    }

    private companion object {
        const val MAX_LEASE_MS = 2_500L
    }
}

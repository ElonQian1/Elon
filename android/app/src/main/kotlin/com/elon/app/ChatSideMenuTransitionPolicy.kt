package com.elon.app

import android.view.View

internal object ChatSideMenuTransitionPolicy {
    const val ANIMATION_DURATION_MS = 260L
    private const val HANDOFF_SETTLE_MS = 32L

    fun closeHandoffDelayMs(animated: Boolean): Long =
        if (animated) ANIMATION_DURATION_MS + HANDOFF_SETTLE_MS else 0L

    fun handoffAfterAnimatedClose(host: View, close: () -> Unit, action: () -> Unit) {
        close()
        host.postDelayed(action, closeHandoffDelayMs(animated = true))
    }
}

package com.elon.app

import android.os.Handler
import android.os.Looper

internal class MainSocialSummaryPolling(
    private val refresh: () -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val runnable = object : Runnable {
        override fun run() {
            refresh()
            handler.postDelayed(this, REFRESH_MS)
        }
    }

    fun start() {
        handler.removeCallbacks(runnable)
        refresh()
        handler.postDelayed(runnable, REFRESH_MS)
    }

    fun stop() {
        handler.removeCallbacks(runnable)
    }

    private companion object {
        const val REFRESH_MS = 8_000L
    }
}

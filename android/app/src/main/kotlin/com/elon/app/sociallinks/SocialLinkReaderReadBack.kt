package com.elon.app.sociallinks

import android.os.Handler
import android.os.Looper

/** One bounded metadata task per navigation. A stale callback cannot write or restart polling. */
internal class SocialLinkReaderReadBack(
    private val handler: Handler = Handler(Looper.getMainLooper()),
    private val capture: (stillCurrent: () -> Boolean, complete: (Boolean) -> Unit) -> Unit,
) {
    private var generation = 0L
    private var started = false
    private var attempts = 0
    private var pending: Runnable? = null
    private var expiry: Runnable? = null

    fun reset() {
        generation++; started = false; attempts = 0
        pending?.let(handler::removeCallbacks); expiry?.let(handler::removeCallbacks)
        pending = null; expiry = null
    }

    fun start() {
        if (started) return
        started = true
        val current = generation
        expiry = Runnable {
            if (generation == current) { generation++; pending?.let(handler::removeCallbacks); pending = null }
        }.also { handler.postDelayed(it, 20000) }
        schedule(current, 750)
    }

    private fun schedule(current: Long, delay: Long) {
        pending = Runnable {
            if (generation != current) return@Runnable
            attempts++
            var completed = false
            capture({ generation == current && !completed }) { success ->
                if (generation != current || completed) return@capture
                completed = true
                if (success || attempts >= 12) {
                    expiry?.let(handler::removeCallbacks); expiry = null
                } else schedule(current, 1500)
            }
        }.also { handler.postDelayed(it, delay) }
    }
}

package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.webkit.WebView

internal class WebChatBackgroundExecutionController(
    private val resumeExecution: () -> Boolean,
    private val pauseExecution: () -> Unit,
    private val isBusy: () -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val idleDelayMs: Long = DEFAULT_IDLE_DELAY_MS,
    private val busyRetryMs: Long = DEFAULT_BUSY_RETRY_MS,
) {
    private var hostActive = false
    private var executionActive = false
    private var scheduledPause: Runnable? = null

    fun hostResumed() {
        hostActive = true
        resumeAndSchedule()
    }

    fun webViewAttached() {
        if (hostActive) resumeAndSchedule()
    }

    fun interactionRequested() {
        // Explicit native actions need a bounded WebView execution lease even when an
        // OEM reports the host as paused while a system or app overlay is visible.
        resumeAndSchedule()
    }

    fun activitySettled() {
        if (hostActive) schedulePause(idleDelayMs)
    }

    fun hostPaused() {
        hostActive = false
        cancelScheduledPause()
        pause()
    }

    private fun resumeAndSchedule() {
        if (!executionActive) executionActive = resumeExecution()
        if (executionActive) schedulePause(idleDelayMs)
    }

    private fun schedulePause(delayMs: Long) {
        cancelScheduledPause()
        lateinit var pause: Runnable
        pause = Runnable {
            if (scheduledPause !== pause) return@Runnable
            scheduledPause = null
            if (isBusy()) schedulePause(busyRetryMs) else pause()
        }
        scheduledPause = pause
        schedule(pause, delayMs)
    }

    private fun pause() {
        if (!executionActive) return
        pauseExecution()
        executionActive = false
    }

    private fun cancelScheduledPause() {
        scheduledPause?.let(cancel)
        scheduledPause = null
    }

    private companion object {
        const val DEFAULT_IDLE_DELAY_MS = 10_000L
        const val DEFAULT_BUSY_RETRY_MS = 5_000L
    }
}

internal fun webChatBackgroundExecutionController(
    webView: () -> WebView?,
    isBusy: () -> Boolean,
): WebChatBackgroundExecutionController {
    val handler = Handler(Looper.getMainLooper())
    return WebChatBackgroundExecutionController(
        resumeExecution = { webView()?.let { it.onResume(); true } ?: false },
        pauseExecution = { webView()?.onPause() },
        isBusy = isBusy,
        schedule = { task, delayMs -> handler.postDelayed(task, delayMs) },
        cancel = handler::removeCallbacks,
    )
}

internal typealias ChatGptBackgroundExecutionController = WebChatBackgroundExecutionController

internal fun chatGptBackgroundExecutionController(
    webView: () -> WebView?,
    isBusy: () -> Boolean,
): ChatGptBackgroundExecutionController = webChatBackgroundExecutionController(webView, isBusy)

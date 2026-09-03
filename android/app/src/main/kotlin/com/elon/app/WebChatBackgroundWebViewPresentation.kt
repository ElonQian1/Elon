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

internal fun WebView.beginWebChatRealtimeVoiceInteraction() {
    beginWebChatBackgroundInteraction()
    isClickable = true
    isFocusable = true
    isFocusableInTouchMode = true
    requestFocusFromTouch()
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

internal enum class WebChatBackgroundInteractionKind {
    TRANSIENT,
    DICTATION_START,
    DICTATION_FINISH,
}

internal class WebChatBackgroundInteractionLease(
    private val webView: () -> WebView?,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
) {
    private var generation = 0L
    private var leasedView: WebView? = null
    private var scheduledRelease: Runnable? = null
    private var interactionKind = WebChatBackgroundInteractionKind.TRANSIENT
    private var dictationCaptureObserved = false

    fun run(action: () -> Unit): Boolean = run(
        WebChatBackgroundInteractionKind.TRANSIENT,
        action,
    )

    fun run(kind: WebChatBackgroundInteractionKind, action: () -> Unit): Boolean {
        val view = webView() ?: return false
        val continuesDictationLease = leasedView === view &&
            interactionKind in DICTATION_KINDS && kind in DICTATION_KINDS
        if (continuesDictationLease) {
            scheduledRelease?.let(cancel)
            scheduledRelease = null
        } else {
            release()
            dictationCaptureObserved = false
        }
        generation += 1
        val expectedGeneration = generation
        leasedView = view
        interactionKind = kind
        if (kind == WebChatBackgroundInteractionKind.TRANSIENT) {
            view.beginWebChatBackgroundInteraction()
        } else {
            view.beginWebChatRealtimeVoiceInteraction()
        }
        view.postOnAnimation {
            if (generation != expectedGeneration || leasedView !== view) return@postOnAnimation
            action()
            scheduleRelease(expectedGeneration, timeoutFor(kind))
        }
        return true
    }

    fun observeDictationState(
        controlActive: Boolean,
        capturePending: Boolean,
        captureActive: Boolean,
    ) {
        if (interactionKind !in DICTATION_KINDS || leasedView == null) return
        when {
            interactionKind == WebChatBackgroundInteractionKind.DICTATION_FINISH &&
                !controlActive && !captureActive -> release()
            captureActive -> {
                dictationCaptureObserved = true
                if (interactionKind == WebChatBackgroundInteractionKind.DICTATION_START) {
                    scheduleRelease(generation, ACTIVE_DICTATION_MAX_MS)
                }
            }
            dictationCaptureObserved && !controlActive && !capturePending -> release()
        }
    }

    fun release() {
        generation += 1
        scheduledRelease?.let(cancel)
        scheduledRelease = null
        leasedView?.let { view ->
            if (interactionKind == WebChatBackgroundInteractionKind.TRANSIENT) {
                view.endWebChatBackgroundInteraction()
            } else {
                view.showWebChatBackgroundSurface()
            }
        }
        leasedView = null
        interactionKind = WebChatBackgroundInteractionKind.TRANSIENT
        dictationCaptureObserved = false
    }

    private fun scheduleRelease(expectedGeneration: Long, delayMs: Long) {
        scheduledRelease?.let(cancel)
        lateinit var releaseTask: Runnable
        releaseTask = Runnable {
            if (scheduledRelease !== releaseTask || generation != expectedGeneration) return@Runnable
            scheduledRelease = null
            this@WebChatBackgroundInteractionLease.release()
        }
        scheduledRelease = releaseTask
        schedule(releaseTask, delayMs)
    }

    private companion object {
        const val TRANSIENT_LEASE_MS = 2_500L
        const val DICTATION_START_LEASE_MS = 10_000L
        const val DICTATION_FINISH_LEASE_MS = 4_000L
        const val ACTIVE_DICTATION_MAX_MS = 5 * 60_000L
        val DICTATION_KINDS = setOf(
            WebChatBackgroundInteractionKind.DICTATION_START,
            WebChatBackgroundInteractionKind.DICTATION_FINISH,
        )

        fun timeoutFor(kind: WebChatBackgroundInteractionKind): Long = when (kind) {
            WebChatBackgroundInteractionKind.TRANSIENT -> TRANSIENT_LEASE_MS
            WebChatBackgroundInteractionKind.DICTATION_START -> DICTATION_START_LEASE_MS
            WebChatBackgroundInteractionKind.DICTATION_FINISH -> DICTATION_FINISH_LEASE_MS
        }
    }
}

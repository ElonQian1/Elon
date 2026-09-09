package com.elon.app

internal class WebChatModelLevelSubmission(private val levelCount: Int) {
    private var trackingTouch = false
    private var pendingTouch: Int? = null

    fun startTouch() {
        trackingTouch = true
        pendingTouch = null
    }

    fun progress(value: Int, fromUser: Boolean): Int? {
        if (!fromUser || value !in 0 until levelCount) {
            pendingTouch = null
            return null
        }
        if (trackingTouch) {
            pendingTouch = value
            return null
        }
        return value
    }

    fun stopTouch(value: Int): Int? {
        val selection = pendingTouch?.takeIf { trackingTouch && it == value }
        trackingTouch = false
        pendingTouch = null
        return selection
    }
}

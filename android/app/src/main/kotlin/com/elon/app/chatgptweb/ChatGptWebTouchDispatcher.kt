package com.elon.app.chatgptweb

import android.os.SystemClock
import android.view.InputDevice
import android.view.MotionEvent
import android.webkit.WebView

internal class ChatGptWebTouchDispatcher(
    private val webView: WebView,
) {
    fun dispatch(request: ChatGptWebEvent.WebTouchRequest, onComplete: (Boolean) -> Unit) {
        webView.post {
            val allowed = request.purpose in ALLOWED_PURPOSES &&
                ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url) &&
                webView.width > 0 && webView.height > 0
            if (!allowed) {
                onComplete(false)
                return@post
            }
            val x = (request.xRatio * webView.width).toFloat().coerceIn(1f, webView.width - 1f)
            val y = (request.yRatio * webView.height).toFloat().coerceIn(1f, webView.height - 1f)
            val downAt = SystemClock.uptimeMillis()
            val down = touchEvent(downAt, downAt, MotionEvent.ACTION_DOWN, x, y)
            val downDispatched = try {
                webView.dispatchTouchEvent(down)
            } finally {
                down.recycle()
            }
            webView.postDelayed({
                val upAt = SystemClock.uptimeMillis()
                val up = touchEvent(downAt, upAt, MotionEvent.ACTION_UP, x, y)
                val upDispatched = try {
                    webView.dispatchTouchEvent(up)
                } finally {
                    up.recycle()
                }
                onComplete(downDispatched && upDispatched)
            }, TAP_DURATION_MS)
        }
    }

    private fun touchEvent(
        downAt: Long,
        eventAt: Long,
        action: Int,
        x: Float,
        y: Float,
    ): MotionEvent = MotionEvent.obtain(downAt, eventAt, action, x, y, 0).apply {
        source = InputDevice.SOURCE_TOUCHSCREEN
    }

    private companion object {
        const val TAP_DURATION_MS = 48L
        val ALLOWED_PURPOSES = setOf(
            "list_model_options",
            "list_composer_tools",
            "select_model_option",
            "select_composer_tool",
            "open_model_submenu",
            "open_composer_tools_submenu",
            "open_model_selector",
            "open_composer_tools",
            "start_dictation",
            "cancel_dictation",
            "submit_dictation",
            "remove_attachment",
            "list_navigation",
            "select_navigation",
            "dismiss_navigation",
            "invoke_ui_control",
            "regenerate_open_menu",
            "regenerate_retry",
        )
    }
}

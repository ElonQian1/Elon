package com.elon.app.chatgptweb

import android.os.SystemClock
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
            val down = MotionEvent.obtain(downAt, downAt, MotionEvent.ACTION_DOWN, x, y, 0)
            val up = MotionEvent.obtain(downAt, downAt + TAP_DURATION_MS, MotionEvent.ACTION_UP, x, y, 0)
            val dispatched = try {
                webView.dispatchTouchEvent(down)
                webView.dispatchTouchEvent(up)
            } finally {
                down.recycle()
                up.recycle()
            }
            onComplete(dispatched)
        }
    }

    private companion object {
        const val TAP_DURATION_MS = 48L
        val ALLOWED_PURPOSES = setOf(
            "list_model_options",
            "list_composer_tools",
            "select_model_option",
            "select_composer_tool",
            "open_model_selector",
            "open_composer_tools",
            "start_dictation",
            "remove_attachment",
            "list_navigation",
            "select_navigation",
            "dismiss_navigation",
            "invoke_ui_control",
        )
    }
}

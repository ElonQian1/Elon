package com.elon.app

import android.view.ViewTreeObserver
import androidx.recyclerview.widget.RecyclerView

internal fun RecyclerView.jumpToLatestMessageBeforeNextDraw() {
    jumpToLatestMessage()
    val observer = viewTreeObserver.takeIf { it.isAlive } ?: return
    observer.addOnPreDrawListener(object : ViewTreeObserver.OnPreDrawListener {
        override fun onPreDraw(): Boolean {
            if (viewTreeObserver.isAlive) {
                viewTreeObserver.removeOnPreDrawListener(this)
            }
            jumpToLatestMessage()
            return true
        }
    })
}

private fun RecyclerView.jumpToLatestMessage() {
    val latestPosition = (adapter?.itemCount ?: 0) - 1
    if (latestPosition < 0) return
    stopScroll()
    scrollToPosition(latestPosition)
}

package com.elon.app

import android.view.View
import android.view.ViewGroup
import android.view.ViewTreeObserver
import java.lang.ref.WeakReference

internal fun View.installScrollIndicatorHider() {
    hideScrollIndicatorsDeep()

    val rootRef = WeakReference(this)
    val hideRunnable = Runnable {
        rootRef.get()?.hideScrollIndicatorsDeep()
    }
    viewTreeObserver.addOnGlobalLayoutListener(object : ViewTreeObserver.OnGlobalLayoutListener {
        override fun onGlobalLayout() {
            val root = rootRef.get() ?: return
            root.removeCallbacks(hideRunnable)
            root.post(hideRunnable)
        }
    })
}

private fun View.hideScrollIndicatorsDeep() {
    isVerticalScrollBarEnabled = false
    isHorizontalScrollBarEnabled = false

    if (this !is ViewGroup) return
    for (index in 0 until childCount) {
        getChildAt(index).hideScrollIndicatorsDeep()
    }
}

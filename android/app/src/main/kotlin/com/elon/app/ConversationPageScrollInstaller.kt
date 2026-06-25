package com.elon.app

import android.content.Context
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ScrollView
import com.elon.app.databinding.ActivityMainBinding

internal fun ActivityMainBinding.ensureConversationPageScrollable() {
    val list = conversationPage
    if (list.parent is ScrollView) return
    val parent = list.parent as? ViewGroup ?: return
    val childIndex = parent.indexOfChild(list)
    val originalParams = list.layoutParams

    parent.removeView(list)

    val scroller = HomeConversationScrollView(list.context).apply {
        layoutParams = originalParams
        isFillViewport = true
        isVerticalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
        clipToPadding = false
        clipChildren = false
    }
    list.layoutParams = FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.WRAP_CONTENT
    )
    scroller.addView(list)

    if (childIndex >= 0) {
        parent.addView(scroller, childIndex)
    } else {
        parent.addView(scroller)
    }
}

internal class HomeConversationScrollView(context: Context) : ScrollView(context) {
    var pullTouchHandler: ((MotionEvent) -> Boolean)? = null
    private var forwardingToPull = false

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        val handledByPull = pullTouchHandler?.invoke(ev) == true
        if (handledByPull) {
            if (!forwardingToPull) {
                forwardingToPull = true
                cancelChildTouch(ev)
            }
            if (ev.actionMasked == MotionEvent.ACTION_UP || ev.actionMasked == MotionEvent.ACTION_CANCEL) {
                forwardingToPull = false
            }
            return true
        }

        if (ev.actionMasked == MotionEvent.ACTION_DOWN ||
            ev.actionMasked == MotionEvent.ACTION_UP ||
            ev.actionMasked == MotionEvent.ACTION_CANCEL
        ) {
            forwardingToPull = false
        }
        return super.dispatchTouchEvent(ev)
    }

    private fun cancelChildTouch(source: MotionEvent) {
        val cancelEvent = MotionEvent.obtain(source).apply {
            action = MotionEvent.ACTION_CANCEL
        }
        super.dispatchTouchEvent(cancelEvent)
        cancelEvent.recycle()
    }
}

internal fun ActivityMainBinding.scrollConversationPageToTop() {
    val scroller = conversationPage.parent as? ScrollView ?: return
    scroller.post {
        scroller.scrollTo(0, 0)
    }
}

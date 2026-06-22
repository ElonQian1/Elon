package com.elon.app

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

    val scroller = ScrollView(list.context).apply {
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

internal fun ActivityMainBinding.scrollConversationPageToTop() {
    val scroller = conversationPage.parent as? ScrollView ?: return
    scroller.post {
        scroller.scrollTo(0, 0)
    }
}

package com.elon.app.grid.share

import android.content.Context
import android.view.MotionEvent
import android.view.View
import android.widget.LinearLayout

/** The message row owns available width (avatar, selection gutter, window and insets). */
internal class GridShareMessageCard(context: Context) : LinearLayout(context) {
    init { orientation = VERTICAL }

    // The summary is one action. Child long-click listeners must not swallow a normal tap.
    // RecyclerView still intercepts a scroll and sends ACTION_CANCEL to this card.
    override fun onInterceptTouchEvent(event: MotionEvent): Boolean = true

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val preferred = (326 * resources.displayMetrics.density).toInt()
        val available = if (View.MeasureSpec.getMode(widthMeasureSpec) == View.MeasureSpec.UNSPECIFIED) preferred
            else minOf(preferred, View.MeasureSpec.getSize(widthMeasureSpec))
        super.onMeasure(View.MeasureSpec.makeMeasureSpec(available, View.MeasureSpec.EXACTLY), heightMeasureSpec)
    }
}

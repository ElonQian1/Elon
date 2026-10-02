package com.elon.app

import android.view.MotionEvent
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

/** A user gesture may request one page. Layout/anchor restoration never starts a scan. */
internal class MessageHistoryScroll(private val list: RecyclerView, private val load: () -> Unit,
    private val loadLatest: (() -> Unit)? = null) {
    private var generation = 0
    var enabled = false
        set(value) { if (field != value) generation++; field = value }
    private var dragging = false
    private var requested = false
    private var downY = 0f
    private fun nearTop(): Boolean = (list.layoutManager as? LinearLayoutManager)
        ?.findFirstVisibleItemPosition()?.let { it in 0..2 } == true
    private fun nearBottom(): Boolean = (list.adapter?.itemCount ?: 0) > 0 && !list.canScrollVertically(1)
    private fun request(latest: Boolean = false) {
        fun atEdge() = if (latest) nearBottom() else nearTop()
        val action = if (latest) loadLatest ?: return else load
        if (enabled && !requested && atEdge()) {
            requested = true
            val ticket = generation
            list.post { if (enabled && ticket == generation && atEdge()) action() }
        }
    }
    init {
        list.addOnScrollListener(object : RecyclerView.OnScrollListener() {
            override fun onScrollStateChanged(view: RecyclerView, state: Int) {
                if (state == RecyclerView.SCROLL_STATE_DRAGGING) { dragging = true; requested = false }
                if (state == RecyclerView.SCROLL_STATE_IDLE) dragging = false
            }
            override fun onScrolled(view: RecyclerView, dx: Int, dy: Int) {
                if (dragging && dy < 0) request()
                if (dragging && dy > 0) request(latest = true)
            }
        })
        list.addOnItemTouchListener(object : RecyclerView.SimpleOnItemTouchListener() {
            override fun onInterceptTouchEvent(view: RecyclerView, event: MotionEvent): Boolean {
                when (event.actionMasked) {
                    MotionEvent.ACTION_DOWN -> { downY = event.y; requested = false }
                    MotionEvent.ACTION_MOVE -> {
                        val distance = event.y - downY
                        val threshold = 24 * list.resources.displayMetrics.density
                        if (distance > threshold) request()
                        if (distance < -threshold) request(latest = true)
                    }
                }
                return false
            }
        })
    }
}

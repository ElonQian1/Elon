package com.elon.app

import android.view.MotionEvent
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

/** A user gesture may request one page. Layout/anchor restoration never starts a scan. */
internal class MessageHistoryScroll(private val list: RecyclerView, private val load: () -> Unit) {
    private var generation = 0
    var enabled = false
        set(value) { if (field != value) generation++; field = value }
    private var dragging = false
    private var requested = false
    private var downY = 0f
    private fun nearTop(): Boolean = (list.layoutManager as? LinearLayoutManager)
        ?.findFirstVisibleItemPosition()?.let { it in 0..2 } == true
    private fun request() {
        if (enabled && !requested && nearTop()) {
            requested = true
            val ticket = generation
            list.post { if (enabled && ticket == generation && nearTop()) load() }
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
            }
        })
        list.addOnItemTouchListener(object : RecyclerView.SimpleOnItemTouchListener() {
            override fun onInterceptTouchEvent(view: RecyclerView, event: MotionEvent): Boolean {
                when (event.actionMasked) {
                    MotionEvent.ACTION_DOWN -> { downY = event.y; requested = false }
                    MotionEvent.ACTION_MOVE -> if (event.y - downY > 24 * list.resources.displayMetrics.density) request()
                }
                return false
            }
        })
    }
}

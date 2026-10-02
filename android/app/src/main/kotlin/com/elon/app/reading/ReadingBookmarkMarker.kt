package com.elon.app

import android.graphics.Rect
import android.view.View
import android.widget.FrameLayout
import android.widget.TextView
import org.json.JSONObject

/** Private metadata only; never written into ChatMessage content or attachment views. */
internal fun readingBookmarkLabels(bookmarks: List<JSONObject>, active: String?, destination: String?): Map<String, String> {
    val labels = linkedMapOf<String, MutableList<String>>()
    for (bookmark in bookmarks) {
        val id = bookmark.optJSONObject("anchor")?.optString("message_id").orEmpty()
        if (id.isNotBlank()) labels.getOrPut(id) { mutableListOf() }.add("书签：${bookmark.optString("title")}")
    }
    bookmarks.find { it.optString("id") == active }?.let { bookmark ->
        if (!destination.isNullOrBlank() && destination != bookmark.optJSONObject("anchor")?.optString("message_id"))
            labels.getOrPut(destination) { mutableListOf() }.add("续读位置：${bookmark.optString("title")}")
    }
    return labels.mapValues { (_, descriptions) -> descriptions.joinToString("；") }
}

/** A sibling overlay preserves bubble measurements, copy sources, and existing row constraints. */
internal class ReadingBookmarkMessageFrame(content: View) : FrameLayout(content.context) {
    private val marker = TextView(context).apply {
        text = "🔖"; textSize = 20f; visibility = View.GONE
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
        isClickable = false; isFocusable = false
        setPadding(0, 0, 0, 0)
    }
    private var own = false
    init {
        layoutParams = content.layoutParams
        addView(content, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT))
        addView(marker, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT))
    }
    fun bind(label: String?, outgoing: Boolean) {
        own = outgoing
        marker.contentDescription = label
        marker.tooltipText = label
        marker.visibility = if (label.isNullOrBlank()) View.GONE else View.VISIBLE
        if (marker.visibility == View.VISIBLE) requestLayout()
    }
    override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
        super.onLayout(changed, left, top, right, bottom)
        positionMarker()
    }
    fun positionMarker() {
        if (marker.visibility != View.VISIBLE || marker.measuredWidth == 0) return
        val bubble = findViewById<View>(R.id.messageBubble) ?: return
        val rect = Rect(0, 0, bubble.width, bubble.height)
        offsetDescendantRectToMyCoords(bubble, rect)
        val gap = (4 * resources.displayMetrics.density).toInt()
        val x = (if (own) rect.left - marker.measuredWidth - gap else rect.right + gap)
            .coerceIn(0, (width - marker.measuredWidth).coerceAtLeast(0))
        // Keep the marker visible when a bookmark resumes partway through a long bubble.
        val visible = Rect(); getLocalVisibleRect(visible)
        val y = maxOf(rect.top, visible.top + gap).coerceAtMost(maxOf(rect.top, rect.bottom - marker.measuredHeight))
        marker.layout(x, y, x + marker.measuredWidth, y + marker.measuredHeight)
    }
}

internal fun bindReadingBookmarkMarker(view: View, message: ChatMessage, labels: Map<String, String>) {
    (view as? ReadingBookmarkMessageFrame)?.bind(message.id?.let(labels::get), message.role == "user")
}

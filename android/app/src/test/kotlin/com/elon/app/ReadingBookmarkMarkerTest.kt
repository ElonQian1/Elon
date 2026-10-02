package com.elon.app

import android.graphics.Rect
import android.view.ContextThemeWrapper
import android.view.View
import android.widget.FrameLayout
import android.widget.TextView
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ReadingBookmarkMarkerTest {
    private fun bookmark(id: String, anchor: String) = JSONObject().put("id", id).put("title", id)
        .put("anchor", JSONObject().put("message_id", anchor))
    @Test fun fixedAnchorsMultipleNamesAndResumeDestinationStaySeparate() {
        val items = listOf(bookmark("A", "start"), bookmark("B", "start"))
        val labels = readingBookmarkLabels(items, "A", "later")
        assertEquals("书签：A；书签：B", labels["start"])
        assertEquals("续读位置：A", labels["later"])
        assertFalse(readingBookmarkLabels(items, "", "later").containsKey("later"))
        assertFalse(readingBookmarkLabels(items.drop(1), "A", "later").containsKey("later"))
        assertTrue(readingBookmarkLabels(emptyList(), null, "later").isEmpty())
    }
    @Test fun realAdapterRendersBesideBothBubbleDirectionsAndClearsRecycledRows() {
        val context = ContextThemeWrapper(RuntimeEnvironment.getApplication(), R.style.Theme_ElonApp)
        for (role in listOf("friend", "user")) {
            val message = ChatMessage(role, "书签旁的合成消息", id = "marked")
            val rows = mutableListOf(message)
            val adapter = ChatAdapter(rows)
            val holder = adapter.onCreateViewHolder(FrameLayout(context), adapter.getItemViewType(0))
            adapter.setReadingBookmarkLabels(mapOf("marked" to "书签：A"))
            adapter.onBindViewHolder(holder, 0)
            val root = holder.itemView as ReadingBookmarkMessageFrame
            root.measure(View.MeasureSpec.makeMeasureSpec(360, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
            root.layout(0, 0, root.measuredWidth, root.measuredHeight)
            val marker = root.getChildAt(1) as TextView
            assertEquals("🔖", marker.text.toString())
            assertEquals("书签：A", marker.contentDescription)
            assertEquals(View.VISIBLE, marker.visibility)
            val bubble = root.findViewById<View>(R.id.messageBubble)
            val rect = Rect(0, 0, bubble.width, bubble.height); root.offsetDescendantRectToMyCoords(bubble, rect)
            assertFalse("marker must not overlap $role bubble", Rect.intersects(rect, Rect(marker.left, marker.top, marker.right, marker.bottom)))
            assertEquals(message.content, holder.text.text.toString())
            rows[0] = ChatMessage(role, "另一条消息", id = "unmarked")
            adapter.onBindViewHolder(holder, 0)
            assertEquals(View.GONE, marker.visibility)
            rows[0] = message
            adapter.onBindViewHolder(holder, 0)
            adapter.setReadingBookmarkLabels(emptyMap())
            adapter.onBindViewHolder(holder, 0, mutableListOf("reading-bookmark"))
            assertEquals(View.GONE, marker.visibility)
            assertEquals(message.content, holder.text.text.toString())
        }
    }
}

package com.elon.app.grid.share

import android.content.res.Configuration
import android.view.ContextThemeWrapper
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.TextView
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.ChatAdapter
import com.elon.app.ChatMessage
import com.elon.app.R
import com.elon.app.socialquotes.SocialQuote
import com.elon.app.socialquotes.SocialQuoteCodec
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

/** Exercise the production message rows, including their avatar and bubble constraints. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GridShareMessageTest {
    private fun message(role: String = "user", id: String = "grid-message"): ChatMessage {
        val grid = GridShareModel.project(mapOf("symbol" to "QNTUSDT", "direction" to "LONG", "leverage" to "10",
            "count" to "30", "spacing" to "ARITH", "status" to "WORKING", "profit" to "479.92684725",
            "unrealizedPnl" to "-301.26979411", "investment" to "683.11265602", "positionQty" to "16.90000000",
            "positionNotional" to "4512.13100000", "lower" to "275.59000000", "upper" to "326.29000000", "markPrice" to "266.99"), 1000)
        val card = GridShareModel.document(grid).put("snapshot_id", "ai_snapshot_test").put("group_id", "group_test")
        return ChatMessage(role, GridShareModel.PREFIX + card, id = id, senderLabel = "分享者")
    }
    private fun context(dark: Boolean = true, scale: Float = 1f): ContextThemeWrapper {
        val base = RuntimeEnvironment.getApplication()
        val config = Configuration(base.resources.configuration).apply {
            fontScale = scale
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or if (dark) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        }
        return ContextThemeWrapper(base.createConfigurationContext(config), R.style.Theme_ElonApp)
    }
    private fun layout(view: View, width: Int) {
        view.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
        view.layout(0, 0, width, view.measuredHeight)
    }
    private fun descendants(view: View): List<View> = listOf(view) + if (view is ViewGroup) (0 until view.childCount).flatMap { descendants(view.getChildAt(it)) } else emptyList()

    @Test fun actualSentAndReceivedRowsNeverClipGridAtNarrowWidthsOrLargeFonts() {
        for (dark in listOf(false, true)) for (scale in listOf(1f, 2f)) for (role in listOf("user", "friend")) {
            val context = context(dark, scale)
            val adapter = ChatAdapter(mutableListOf(message(role)))
            val holder = adapter.onCreateViewHolder(FrameLayout(context), adapter.getItemViewType(0))
            adapter.onBindViewHolder(holder, 0)
            for (width in listOf(320, 390, 280)) {
                layout(holder.itemView, (width * context.resources.displayMetrics.density).toInt())
                descendants(holder.attachmentList!!).filter { it.visibility == View.VISIBLE }.forEach { child ->
                    val parent = child.parent as? View ?: return@forEach
                    assertTrue("clipped ${child.javaClass.simpleName} ${(child as? TextView)?.text}: $role width=$width scale=$scale right=${child.right} parent=${parent.width}", child.left >= 0 && child.right <= parent.width)
                    if (child is TextView && child.text.isNotBlank()) {
                        assertTrue(child.layout.lineCount > 0)
                        assertEquals("truncated ${child.text}", child.text.length, child.layout.getLineEnd(child.layout.lineCount - 1))
                        for (line in 0 until child.layout.lineCount) assertEquals(0, child.layout.getEllipsisCount(line))
                    }
                }
                assertTrue(descendants(holder.attachmentList!!).filterIsInstance<TextView>().any { it.text == "查看网格详情 ›" && it.height >= (48 * context.resources.displayMetrics.density).toInt() })
            }
        }
    }

    @Test fun longPressUsesCurrentMessageAndSelectionKeepsCardActionsSuppressed() {
        val context = context()
        val rows = mutableListOf(message())
        var selected: ChatMessage? = null
        val adapter = ChatAdapter(rows, onMessageLongPress = { _, row -> selected = row })
        val recycler = RecyclerView(context).apply { layoutManager = LinearLayoutManager(context); this.adapter = adapter }
        val activity = org.robolectric.Robolectric.buildActivity(android.app.Activity::class.java).setup().get()
        activity.setContentView(recycler)
        recycler.measure(View.MeasureSpec.makeMeasureSpec(390, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(2000, View.MeasureSpec.EXACTLY))
        recycler.layout(0, 0, 390, 2000)
        val holder = recycler.findViewHolderForAdapterPosition(0) as ChatAdapter.VH
        val card = holder.attachmentList!!.getChildAt(0)
        var opened = 0
        card.setOnClickListener { opened++ }
        // Child long-click listeners must not swallow a normal tap on a metric.
        val now = android.os.SystemClock.uptimeMillis()
        for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
            val event = MotionEvent.obtain(now, now + action * 30L, action, card.width / 2f, card.height / 2f, 0)
            card.dispatchTouchEvent(event); event.recycle()
        }
        org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
        assertEquals(1, opened)
        val metric = descendants(holder.attachmentList!!).filterIsInstance<TextView>().single { it.text == "+479.92684725" }
        assertTrue(metric.performLongClick()); assertSame(rows[0], selected)
        rows[0] = message(id = "replacement"); adapter.onBindViewHolder(holder, 0)
        assertTrue(holder.attachmentList!!.getChildAt(0).performLongClick()); assertSame(rows[0], selected)
        adapter.startSelection(rows[0])
        // Let RecyclerView consume notifyDataSetChanged and restore adapter positions.
        layout(recycler, 390)
        val selectedHolder = recycler.findViewHolderForAdapterPosition(0) as ChatAdapter.VH
        assertFalse(selectedHolder.attachmentList!!.getChildAt(0).isLongClickable)
        assertEquals(View.VISIBLE, selectedHolder.selectionCheck!!.visibility)
        assertEquals("✓", selectedHolder.selectionCheck!!.text.toString())
        assertTrue(selectedHolder.attachmentList!!.getChildAt(0).performClick())
        assertTrue(adapter.selectedMessagesInOrder().isEmpty())
        adapter.onBindViewHolder(selectedHolder, 0)
        assertEquals("", selectedHolder.selectionCheck!!.text.toString())
        assertEquals("[网格快照] QNTUSDT · 做多 10×", SocialQuoteCodec.summary(SocialQuoteCodec.from(rows[0], "我")))
        assertEquals("原消息已撤回或不可用", SocialQuoteCodec.summary(SocialQuote("id", "分享者", rows[0].content, unavailable = true)))
    }
}

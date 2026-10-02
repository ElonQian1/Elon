package com.elon.app.chatrecords

import android.app.Activity
import android.app.Application
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.ChatMessage
import com.elon.app.bindChatSelectionContent
import com.elon.app.bindChatSelectionLongPress
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.time.Duration

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordCardInteractionTest {
    @Test fun titleSummaryAndFooterOpenReaderWithRealTouchUnlessSelecting() {
        verifyCardInteractions(false)
    }
    @Test fun groupLongPressDoesNotConsumeTitleSummaryFooterOrPaddingTaps() {
        verifyCardInteractions(true)
    }
    private fun verifyCardInteractions(withLongPress: Boolean) {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val root = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        activity.setContentView(root)
        val json = JSONObject().put("schema", "chat_record_bundle_v1").put("group_id", "group_test")
            .put("record_id", "record_test").put("title", "记录").put("summary", "测试摘要").put("message_count", 2)
        val message = ChatMessage(role = "friend", content = "【一龙聊天记录】\n$json")
        var held = 0
        val listener = if (withLongPress) View.OnLongClickListener { held++; true } else null
        fun bind() {
            assertTrue(ChatRecordCardViews.bind(root, TextView(activity), message))
            bindChatSelectionLongPress(root, listener)
            layout(root)
        }
        bind()
        var card = root.getChildAt(0) as LinearLayout
        fun assertOpened() {
            val intent = shadowOf(activity).nextStartedActivity
            assertEquals(ChatRecordReaderActivity::class.java.name, intent?.component?.className)
            assertEquals("group_test", intent?.getStringExtra("group"))
            assertEquals("record_test", intent?.getStringExtra("record"))
            assertNull("A tap must open only one reader", shadowOf(activity).nextStartedActivity)
        }
        for (index in 0 until card.childCount) {
            val child = card.getChildAt(index)
            assertEquals(withLongPress, child.isLongClickable)
            touch(root, child); assertOpened()
            if (withLongPress) {
                touch(root, child, long = true)
                assertEquals(index + 1, held)
                assertNull("Long press must not open the reader", shadowOf(activity).nextStartedActivity)
            }
        }
        touch(root, card, padding = true); assertOpened()
        var selected = 0
        bindChatSelectionContent(root, View.OnClickListener { selected++ })
        for (index in 0 until card.childCount) {
            touch(root, card.getChildAt(index))
            assertEquals(index + 1, selected)
            assertNull(shadowOf(activity).nextStartedActivity)
        }
        bind()
        card = root.getChildAt(0) as LinearLayout
        for (index in 0 until card.childCount) { touch(root, card.getChildAt(index)); assertOpened() }
    }
    private fun layout(root: View) {
        root.measure(View.MeasureSpec.makeMeasureSpec(390, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(844, View.MeasureSpec.EXACTLY))
        root.layout(0, 0, 390, 844)
    }
    private fun touch(root: View, target: View, long: Boolean = false, padding: Boolean = false) {
        val p = IntArray(2); val o = IntArray(2); target.getLocationOnScreen(p); root.getLocationOnScreen(o)
        assertTrue(target.height > 0)
        val now = SystemClock.uptimeMillis()
        val x = p[0] - o[0] + if (padding) 1f else target.width / 2f
        val y = p[1] - o[1] + if (padding) 1f else target.height / 2f
        for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
            if (long && action == MotionEvent.ACTION_UP) shadowOf(android.os.Looper.getMainLooper()).idleFor(Duration.ofMillis(700))
            val event = MotionEvent.obtain(now, SystemClock.uptimeMillis(), action, x, y, 0)
            root.dispatchTouchEvent(event); event.recycle()
        }
        shadowOf(android.os.Looper.getMainLooper()).idle()
    }
}

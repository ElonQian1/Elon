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

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordCardInteractionTest {
    @Test fun titleSummaryAndFooterOpenReaderWithRealTouchUnlessSelecting() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val root = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        activity.setContentView(root)
        val json = JSONObject().put("schema", "chat_record_bundle_v1").put("group_id", "group_test")
            .put("record_id", "record_test").put("title", "记录").put("summary", "测试摘要").put("message_count", 2)
        assertTrue(ChatRecordCardViews.bind(root, TextView(activity), ChatMessage(role = "friend", content = "【一龙聊天记录】\n$json")))
        bindChatSelectionLongPress(root, null)
        root.measure(View.MeasureSpec.makeMeasureSpec(390, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(844, View.MeasureSpec.EXACTLY))
        root.layout(0, 0, 390, 844)
        val card = root.getChildAt(0) as LinearLayout
        for (index in 0 until card.childCount) {
            val child = card.getChildAt(index)
            assertFalse(child.isLongClickable)
            touch(root, child)
            assertEquals(ChatRecordReaderActivity::class.java.name, shadowOf(activity).nextStartedActivity?.component?.className)
        }
        var selected = false
        bindChatSelectionContent(root, View.OnClickListener { selected = true })
        touch(root, card.getChildAt(0)); assertTrue(selected); assertNull(shadowOf(activity).nextStartedActivity)
    }
    private fun touch(root: View, target: View) {
        val p = IntArray(2); val o = IntArray(2); target.getLocationOnScreen(p); root.getLocationOnScreen(o)
        assertTrue(target.height > 0)
        val now = SystemClock.uptimeMillis(); val x = p[0] - o[0] + target.width / 2f; val y = p[1] - o[1] + target.height / 2f
        for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
            val event = MotionEvent.obtain(now, now + action * 50, action, x, y, 0)
            root.dispatchTouchEvent(event); event.recycle()
        }
        shadowOf(android.os.Looper.getMainLooper()).idle()
    }
}

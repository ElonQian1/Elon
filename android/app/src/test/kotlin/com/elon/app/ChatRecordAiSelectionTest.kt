package com.elon.app

import android.view.View
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatRecordAiSelectionTest {
    @Test fun importedRecordLongPressAndSelectionKeepServerIdentity() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
            val activity = controller.get()
            val content = "【一龙聊天记录】\n" + """{"schema":"chat_record_bundle_v1","record_id":"record_test","group_id":"group_test","title":"微信聊天记录","summary":"discussion","message_count":2,"total_count":3}"""
            val message = ChatMessage("friend", content, id = "record_message", revision = 7)
            lateinit var adapter: ChatAdapter
            adapter = ChatAdapter(mutableListOf(message), onMessageLongPress = { _, row -> adapter.startSelection(row) })
            val list = RecyclerView(activity).apply {
                layoutManager = LinearLayoutManager(activity)
                itemAnimator = null
                this.adapter = adapter
            }
            activity.setContentView(list)
            fun layoutRows() {
                list.requestLayout()
                list.measure(View.MeasureSpec.makeMeasureSpec(360, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(800, View.MeasureSpec.EXACTLY))
                list.layout(0, 0, 360, 800)
            }
            fun holder() = list.findViewHolderForAdapterPosition(0) as ChatAdapter.VH
            layoutRows()
            val card = holder().attachmentList!!.getChildAt(0)
            assertEquals("查看聊天记录 微信聊天记录", card.contentDescription)
            assertTrue(card.performLongClick())
            layoutRows()
            assertTrue(adapter.isSelectionModeActive())
            assertEquals("record_message", adapter.selectedMessagesInOrder().single().id)
            assertEquals(7L, adapter.selectedMessagesInOrder().single().revision)
            assertEquals(content, adapter.selectedMessagesInOrder().single().content)
            assertEquals(View.VISIBLE, holder().selectionCheck!!.visibility)
            assertTrue(holder().itemView.performClick())
            layoutRows()
            assertTrue(adapter.selectedMessagesInOrder().isEmpty())
        } finally { controller.pause().stop().destroy() }
    }
}

package com.elon.app

import android.app.Application
import android.view.ContextThemeWrapper
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordMessageMenuTest {
    private fun record(id: String = "record") = ChatMessage("friend", "【一龙聊天记录】\n" + JSONObject()
        .put("schema", "chat_record_bundle_v1").put("group_id", "test-group").put("record_id", "test-record")
        .put("title", "测试聊天记录").put("summary", "合成资料").put("message_count", 2), id = id)
    private fun views(root: View): List<View> = listOf(root) + if (root is ViewGroup)
        (0 until root.childCount).flatMap { views(root.getChildAt(it)) } else emptyList()

    @Test fun fullAdapterPreservesRecordMenuOnAllLabelsAndRecycling() {
        val context = ContextThemeWrapper(RuntimeEnvironment.getApplication(), R.style.Theme_ElonApp)
        val messages = mutableListOf(record())
        var chosen: ChatMessage? = null
        val adapter = ChatAdapter(messages, onMessageLongPress = { _, message -> chosen = message })
        val holder = adapter.onCreateViewHolder(FrameLayout(context), adapter.getItemViewType(0))
        for (id in listOf("first", "recycled")) {
            messages[0] = record(id); adapter.onBindViewHolder(holder, 0)
            val labels = views(holder.attachmentList!!).filterIsInstance<TextView>()
            assertTrue(labels.size >= 3)
            labels.forEach { label ->
                assertTrue(label.isClickable)
                assertTrue(label.performLongClick())
                assertSame(messages[0], chosen)
            }
        }
    }

    @Test fun commonMenuAndReadingActionsAppearTogetherWithoutIntermediateMenu() {
        val controller = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = controller.get().apply { setTheme(R.style.Theme_ElonApp) }
        controller.setup()
        val binding = ActivityMainBinding.inflate(activity.layoutInflater)
        activity.setContentView(binding.root)
        var popup: PopupWindow? = null
        var quoted: ChatMessage? = null
        val message = record()
        val actions = MainActionPopups(activity, binding, { popup }, { popup = it }, { error("Unused sharing") },
            {}, {}, {}, {}, {}, {}, {}, {}, { true }, {}, {}, {}, { _, _ -> }, { quoted = it }, {}, { true }, {},
            { it }, { null }, {}, additionalMessageActions = { listOf(TopAction("阅读书签", R.drawable.ic_msg_favorite) {}) })
        actions.showMessageActionPopup(binding.inputEdit, message, message.content)
        val labels = views(popup!!.contentView).filterIsInstance<TextView>().associateBy { it.text.toString() }
        for (label in listOf("引用", "多选", "时间", "撤回", "删除", "AI分析记录", "阅读书签", "打开记录", "复制标题"))
            assertTrue("Missing $label", labels.containsKey(label))
        assertFalse(labels.containsKey("其他消息操作"))
        assertFalse(labels.containsKey("复制"))
        assertFalse(labels.containsKey("转发"))
        (labels.getValue("引用").parent as View).performClick()
        assertSame(message, quoted)
        popup?.dismiss(); controller.pause().stop().destroy()
    }
}

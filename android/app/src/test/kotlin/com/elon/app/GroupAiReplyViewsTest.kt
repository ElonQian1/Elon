package com.elon.app

import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w360dp-h800dp-mdpi", application = android.app.Application::class)
class GroupAiReplyViewsTest {
    @Test fun footerIsInsideAnswerAndSourcesAreBelowWithoutDuplicateControls() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
            val message = ChatMessage("ai", "Answer", id = "gai_fixture", groupAiReply = """
                {"schema":1,"provider":"chatgpt_web","requester_id":"fixture-owner","source_count":2,
                 "previews":[{"sender_name":"A","text":"First"},{"sender_name":"B","text":"Second"}]}
            """.trimIndent())
            val adapter = ChatAdapter(mutableListOf(message))
            val holder = adapter.onCreateViewHolder(FrameLayout(host.get()), adapter.getItemViewType(0))
            val actions = mutableListOf<String>()
            repeat(2) { GroupAiReplyViews.bind(holder, message) { _, action -> actions.add(action) } }
            val bubble = holder.bubble!!
            val footer = bubble.findViewWithTag<ViewGroup>("group-ai-continue")!!
            assertSame(bubble, footer.parent)
            footer.getChildAt(0).performClick()
            val source = holder.itemView.findViewWithTag<View>("group-ai-sources")!!
            assertSame(bubble.parent, source.parent)
            source.performClick()
            assertEquals(listOf("continue", "sources"), actions)
            val parent = bubble.parent as ViewGroup
            assertEquals(1, (0 until parent.childCount).count { parent.getChildAt(it).tag == "group-ai-sources" })
            message.recalledAt = "recalled"
            GroupAiReplyViews.bind(holder, message) { _, action -> actions.add(action) }
            assertNull(bubble.findViewWithTag<View>("group-ai-continue"))
            assertNull(holder.itemView.findViewWithTag<View>("group-ai-sources"))
        } finally { host.pause().stop().destroy() }
    }
}

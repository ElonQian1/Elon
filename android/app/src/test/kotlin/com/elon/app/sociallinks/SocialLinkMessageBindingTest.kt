package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.view.View
import android.widget.FrameLayout
import com.elon.app.ChatAdapter
import com.elon.app.ChatAttachment
import com.elon.app.ChatMessage
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class SocialLinkMessageBindingTest {
    @Test fun recycledMessageRestoresTextBubbleAndPreservesCommentsAndAttachments() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val original = "https://mp.weixin.qq.com/s/test"
        for (role in listOf("user", "friend")) {
            val messages = mutableListOf(ChatMessage(role, original))
            val adapter = ChatAdapter(messages)
            // Unattached holder exercises the real binding path without requesting network previews.
            val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
            val bubble = requireNotNull(holder.bubble); val attachments = requireNotNull(holder.attachmentList)
            adapter.onBindViewHolder(holder, 0)
            assertEquals(View.GONE, holder.text.visibility)
            assertEquals(original, holder.text.text.toString())
            assertEquals(Color.TRANSPARENT, (bubble.background as ColorDrawable).color)
            assertEquals(0, bubble.paddingLeft)
            assertEquals(1, attachments.childCount)

            messages[0] = ChatMessage(role, "原来的普通消息")
            adapter.onBindViewHolder(holder, 0)
            assertEquals(View.VISIBLE, holder.text.visibility)
            assertEquals("原来的普通消息", holder.text.text.toString())
            assertTrue(bubble.paddingLeft > 0)
            assertFalse(bubble.background is ColorDrawable)
            assertEquals(0, attachments.childCount)

            for (message in listOf(ChatMessage(role, "我的评论 $original"),
                ChatMessage(role, original, attachments = listOf(ChatAttachment(kind = "file", displayName = "说明.txt"))))) {
                messages[0] = message
                adapter.onBindViewHolder(holder, 0)
                assertEquals(View.VISIBLE, holder.text.visibility)
                assertEquals(message.content, holder.text.text.toString())
                assertTrue(bubble.paddingLeft > 0)
                assertEquals(if (message.attachments.isNullOrEmpty()) 1 else 2, attachments.childCount)
            }
        }
    }
}

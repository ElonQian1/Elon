package com.elon.app

import android.app.Activity
import android.graphics.Rect
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.TextView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w360dp-h800dp-mdpi", application = android.app.Application::class)
class ChatSenderNameTest {
    @Test fun resolvesSocialAiAndPreservesSharedProviderIdentity() {
        assertEquals("一龙ai EL", chatSenderName(ChatMessage("ai", "", senderLabel = "EL", senderUserId = "usr_elon_ai"), "Me"))
        assertEquals("一龙ai EL", chatSenderName(ChatMessage("ai", ""), "Me"))
        assertEquals("ChatGPT", chatSenderName(ChatMessage("friend", "", senderLabel = "ChatGPT"), "Me"))
        assertEquals("Author", chatSenderName(ChatMessage("user", "", senderLabel = " Author "), "Viewer"))
        assertEquals("Me", chatSenderName(ChatMessage("user", ""), "Me"))
        assertEquals("群成员", chatSenderName(ChatMessage("friend", "", senderLabel = "null"), "Me"))
        assertNull(chatSenderName(ChatMessage("ai-progress", ""), "Me"))
    }

    @Test fun namesStayAboveBubblesAndInsideSmallScreensAfterRecycling() {
        val host = Robolectric.buildActivity(Activity::class.java).setup()
        try {
            val activity = host.get()
            activity.setTheme(R.style.Theme_ElonApp)
            UserProfileStore.save(activity, "My nickname", null, "")
            for (role in listOf("user", "friend", "ai")) {
                val message = ChatMessage(role, "Hello", senderLabel = if (role == "friend") "A very long sender name ".repeat(8) else null)
                val adapter = ChatAdapter(mutableListOf(message))
                val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
                adapter.onBindViewHolder(holder, 0)
                val root = holder.itemView as ViewGroup
                val label = root.findViewById<TextView>(R.id.messageSenderName)
                assertEquals(View.VISIBLE, label.visibility)
                for (width in listOf(320, 360, 640)) {
                    root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                    root.layout(0, 0, width, root.measuredHeight)
                    fun bounds(view: View) = Rect(0, 0, view.width, view.height).also { root.offsetDescendantRectToMyCoords(view, it) }
                    val nameBounds = bounds(label)
                    val bubbleBounds = bounds(requireNotNull(holder.bubble))
                    assertTrue("name above $role bubble at $width", nameBounds.bottom <= bubbleBounds.top)
                    assertTrue("name inside $width", nameBounds.left >= 0 && nameBounds.right <= width)
                    if (role == "user") assertEquals(nameBounds.right, bubbleBounds.right)
                    else assertEquals(nameBounds.left, bubbleBounds.left)
                }
                message.senderLabel = "Next sender"
                adapter.onBindViewHolder(holder, 0)
                assertEquals("Next sender", label.text.toString())
            }
        } finally { host.pause().stop().destroy() }
    }
}

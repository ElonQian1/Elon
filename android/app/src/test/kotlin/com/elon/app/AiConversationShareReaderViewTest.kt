package com.elon.app

import android.os.Bundle
import android.os.Looper
import android.view.View
import android.widget.EditText
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class AiConversationShareReaderViewTest {
    @Test fun fullScreenDialogPreservesHostDraftAndDoesNotPauseActivity() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val activity = controller.get()
        val draft = EditText(activity).apply { setText("unsent group draft") }
        activity.setContentView(draft)
        var dismissed = false
        val screen = AiConversationShareReaderView(activity, card(), {}, { dismissed = true }, null)
        screen.show()
        assertTrue(screen.dialog.isShowing)
        assertEquals(0, activity.pauses)
        assertEquals("unsent group draft", draft.text.toString())
        assertNull(screen.dialog.findViewById<View>(R.id.inputEdit))
        screen.dialog.findViewById<View>(R.id.ai_conversation_share_back)!!.performClick()
        // Dialog posts its OnDismissListener callback to the main looper.
        shadowOf(Looper.getMainLooper()).idle()
        assertTrue(dismissed)
        assertFalse(screen.dialog.isShowing)
        assertEquals("unsent group draft", draft.text.toString())
        controller.pause().stop().destroy()
    }

    @Test fun readOnlyBubbleUsesSharerAndHidesEveryActionBar() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val activity = controller.get()
        val row = ChatMessage("user", "question", senderLabel = "Shared Author", modelUsed = "model",
            apkUrl = "https://example.com/app.apk", sendStatus = "failed")
        val adapter = ChatAdapter(mutableListOf(row), readOnly = true)
        val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)
        assertEquals("Shared Author", holder.userAvatar!!.contentDescription)
        assertEquals(View.GONE, holder.status!!.visibility)
        assertEquals(View.GONE, holder.selectionCheck!!.visibility)
        assertEquals(View.GONE, holder.itemView.findViewById<View>(R.id.webChatMessageActionBar).visibility)
        adapter.startSelection(row)
        assertFalse(adapter.isSelectionModeActive())
        controller.pause().stop().destroy()
    }

    @Test fun adapterSelectionMapsCurrentDeleteIndicesButReturnsSelectedRevision() {
        val rows = mutableListOf(ChatMessage("friend", "first", id = "first"),
            ChatMessage("friend", "selected", id = "selected", revision = 7))
        val adapter = ChatAdapter(rows)
        adapter.startSelection(rows[1])
        rows.add(0, ChatMessage("friend", "history", id = "history"))
        adapter.notifyItemInserted(0)
        rows[2] = rows[2].copy(content = "streamed", revision = 8)
        adapter.notifyMessageUpdated(2)
        assertEquals(listOf(2), adapter.selectedPositionsDescending())
        assertEquals("selected", adapter.selectedMessagesInOrder().single().content)
        assertEquals(7L, adapter.selectedMessageIdentitiesInOrder().single().revision)
        val copy = adapter.currentMessagesForSharing()
        rows[2].content = "changed again"
        assertEquals("streamed", copy[2].content)
    }

    @Test fun deselectingLastRowKeepsSelectionModeAndAllowsAnotherRowUntilCancel() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
            val activity = controller.get()
            val rows = mutableListOf(ChatMessage("friend", "first", id = "first"),
                ChatMessage("friend", "second", id = "second"))
            lateinit var adapter: ChatAdapter
            adapter = ChatAdapter(rows, onMessageLongPress = { _, message -> adapter.startSelection(message) })
            val headerCounts = mutableListOf<Int>()
            adapter.setSelectionChangedListener { headerCounts.add(it) }
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
            fun row(index: Int) = requireNotNull(list.findViewHolderForAdapterPosition(index) as? ChatAdapter.VH)

            layoutRows()
            assertTrue(row(0).itemView.performLongClick())
            layoutRows()
            assertTrue(adapter.isSelectionModeActive())
            assertEquals(listOf("first"), adapter.selectedMessagesInOrder().map { it.id })
            assertEquals(1, headerCounts.last())

            assertTrue(row(0).itemView.performClick())
            layoutRows()
            assertTrue("zero selected must not exit the header's selection mode", adapter.isSelectionModeActive())
            assertTrue(adapter.selectedMessagesInOrder().isEmpty())
            assertEquals(0, headerCounts.last())
            assertEquals(View.VISIBLE, row(0).selectionCheck!!.visibility)
            assertEquals("", row(0).selectionCheck!!.text.toString())
            assertEquals(View.VISIBLE, row(1).selectionCheck!!.visibility)

            assertTrue(row(1).itemView.performClick())
            layoutRows()
            assertTrue(adapter.isSelectionModeActive())
            assertEquals(listOf("second"), adapter.selectedMessagesInOrder().map { it.id })
            assertEquals(1, headerCounts.last())
            assertTrue(row(1).selectionCheck!!.text.isNotBlank())

            adapter.exitSelection()
            layoutRows()
            assertFalse(adapter.isSelectionModeActive())
            assertEquals(0, headerCounts.last())
            assertEquals(View.GONE, row(1).selectionCheck!!.visibility)
        } finally {
            controller.pause().stop().destroy()
        }
    }

    @Test fun questionMarkdownUsesRichBinderWithoutChangingRightBubbleOrAuthor() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val activity = controller.get()
        val row = ChatMessage("user", "**Question**", senderLabel = "Shared Author", id = "question")
        val adapter = ChatAdapter(mutableListOf(row), readOnly = true)
        assertEquals(0, adapter.getItemViewType(0))
        val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)
        assertEquals("Question", holder.text.text.toString().trim())
        assertNotNull(holder.userAvatar)
        assertNull(holder.friendAvatar)
        assertEquals("Shared Author", holder.userAvatar!!.contentDescription)
        assertEquals("user", row.role)
        assertEquals("**Question**", row.content)
        controller.pause().stop().destroy()
    }

    @Test fun richMarkdownLinksKeepOnlyPublicHttpTargetsAndPreserveText() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val activity = controller.get()
        val row = ChatMessage("friend", "[docs](https://developer.android.com/guide) [private](file:///secret)", id = "links")
        val adapter = ChatAdapter(mutableListOf(row), readOnly = true)
        val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)
        val rendered = holder.text.text as android.text.Spanned
        assertEquals("docs private", rendered.toString().trim())
        assertEquals(1, rendered.getSpans(0, rendered.length, android.text.style.ClickableSpan::class.java).size)
        controller.pause().stop().destroy()
    }

    @Test fun denialClearRemovesRowsAndSystemBackClosesReader() {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val activity = controller.get()
        val screen = AiConversationShareReaderView(activity, card(), {}, {}, {})
        screen.show()
        val snapshot = AiConversationShareSnapshot(card(), listOf(ChatMessage("friend", "private text", id = "row")))
        val rows = AiConversationShareReaderPresentation.messages(snapshot)
        screen.render(snapshot, rows, ChatAdapter(rows.toMutableList(), readOnly = true), null)
        assertEquals(1, screen.list.adapter!!.itemCount)
        screen.clear()
        screen.showNotice(R.string.ai_conversation_share_revoked)
        assertNull(screen.list.adapter)
        @Suppress("DEPRECATION")
        screen.dialog.onBackPressed()
        assertFalse(screen.dialog.isShowing)
        controller.pause().stop().destroy()
    }

    private fun card() = AiConversationShareCard("ai_snapshot_test", "group", "Shared title", "Excerpt", "chatgpt", "Shared Author", 1)
}

class AiConversationShareReaderTestActivity : AppCompatActivity() {
    var pauses = 0
    override fun onCreate(savedInstanceState: Bundle?) {
        setTheme(R.style.Theme_ElonApp)
        super.onCreate(savedInstanceState)
    }
    override fun onPause() { pauses++; super.onPause() }
}

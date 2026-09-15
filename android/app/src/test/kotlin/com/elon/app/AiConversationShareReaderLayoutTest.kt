package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Rect
import android.os.Looper
import android.text.Spanned
import android.text.style.MetricAffectingSpan
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/** Native view measurement assertions, not screenshot or physical-device acceptance. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w360dp-h800dp-mdpi", application = android.app.Application::class)
class AiConversationShareReaderLayoutTest {
    @Test fun measuredQuestionStaysRightAndShowsSharerInsteadOfSavedViewer() = withHost { activity ->
        UserProfileStore.save(activity, "Viewer Only", null, "viewer signature")
        val row = ChatMessage("user", "**Shared question**", id = "question", senderLabel = "Shared Author")
        val adapter = ChatAdapter(mutableListOf(row), readOnly = true)
        val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)

        listOf(320, 360, 640).forEach { width ->
            measureRow(holder.itemView, width)
            val root = holder.itemView as ViewGroup
            val avatar = requireNotNull(holder.userAvatar)
            val avatarBounds = bounds(root, avatar)
            val bubbleBounds = bounds(root, requireNotNull(holder.bubble))
            assertTrue("user avatar must stay in the right half at $width px", avatarBounds.centerX() > width / 2)
            assertTrue("bubble must not overlap avatar", bubbleBounds.right <= avatarBounds.left)
            assertTrue("bubble has visible width", bubbleBounds.width() > 0)
            assertTrue("bubble stays within row", bubbleBounds.left >= 0 && avatarBounds.right <= width)
            assertEquals("Shared Author", avatar.contentDescription)
            assertEquals("S", avatar.text.toString())
            assertNotEquals(UserProfileStore.avatarInitial(UserProfileStore.load(activity).displayName), avatar.text.toString())
        }
        val rich = holder.text.text as Spanned
        assertEquals("Shared question", rich.toString().trim())
        assertTrue("bold markdown must remain styled", rich.getSpans(0, rich.length, MetricAffectingSpan::class.java).isNotEmpty())
        assertEquals("**Shared question**", row.content)
        assertEquals("user", row.role)
    }

    @Test fun measuredDialogHasOnlySearchInputAndBackPreservesParentState() = withHost { activity ->
        val draft = EditText(activity).apply { setText("unsent group draft"); setSelection(5) }
        val history = ScrollView(activity).apply {
            addView(TextView(activity).apply { text = "group history"; minimumHeight = 2400 })
        }
        val parent = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(history, LinearLayout.LayoutParams(-1, 320))
            addView(draft, LinearLayout.LayoutParams(-1, 56))
        }
        activity.setContentView(parent)
        measurePage(parent, 360, 800)
        history.scrollTo(0, 144)
        val scrollBefore = history.scrollY
        assertTrue(scrollBefore > 0)
        val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("before-reader", "unchanged clipboard"))

        listOf(false, true).forEach { useSystemBack ->
            var closes = 0
            val screen = AiConversationShareReaderView(activity, card(), {}, { closes++ }, null)
            screen.show()
            val snapshot = AiConversationShareSnapshot(card(), listOf(
                ChatMessage("user", "**Question**", id = "q", senderLabel = "Shared Author"),
                ChatMessage("friend", "Answer", id = "a", senderLabel = "ChatGPT")))
            val rows = AiConversationShareReaderPresentation.messages(snapshot)
            screen.render(snapshot, rows, ChatAdapter(rows.toMutableList(), readOnly = true), null)
            val root = screen.dialog.findViewById<ViewGroup>(R.id.ai_conversation_share_reader)!!
            listOf(320 to 720, 640 to 360).forEach { (width, height) ->
                measurePage(root, width, height)
                assertTrue("reader list must have visible space", screen.list.width > 0 && screen.list.height > 0)
                val question = screen.list.findViewHolderForAdapterPosition(0) as? ChatAdapter.VH
                assertNotNull("RecyclerView must lay out the real question row", question)
                assertEquals("Question", question!!.text.text.toString().trim())
                assertEquals(listOf(R.id.ai_conversation_share_search), descendants(root).filterIsInstance<EditText>().map { it.id })
                listOf(R.id.inputLayout, R.id.inputEdit, R.id.sendButton, R.id.modelButton, R.id.voiceCallButton).forEach {
                    assertNull("no live composer/control $it", root.findViewById<View>(it))
                }
                descendants(root).filter { it.id == R.id.webChatMessageCopy }.forEach {
                    assertFalse("read-only rows have no visible clipboard action", it.isShown)
                }
                assertEquals(0, activity.pauses)
            }
            if (useSystemBack) {
                @Suppress("DEPRECATION")
                screen.dialog.onBackPressed()
            } else root.findViewById<View>(R.id.ai_conversation_share_back).performClick()

            // Dialog posts its OnDismissListener callback to the main looper.
            shadowOf(Looper.getMainLooper()).idle()
            assertEquals(1, closes)
            assertFalse(screen.dialog.isShowing)
            assertFalse(activity.isFinishing)
            assertFalse(activity.isDestroyed)
            assertEquals(0, activity.pauses)
            assertSame(parent, activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0))
            assertEquals("unsent group draft", draft.text.toString())
            assertEquals(5, draft.selectionStart)
            assertEquals(scrollBefore, history.scrollY)
            assertEquals("unchanged clipboard", clipboard.primaryClip!!.getItemAt(0).text.toString())
        }
    }

    @Test fun measuredGroupCardBindsMetadataAndOpensExactlyItsSnapshot() = withHost { activity ->
        UserProfileStore.save(activity, "Viewer Only", null, "")
        val shared = card().copy(title = "Shared title ".repeat(8).trim(), summary = "Selected excerpt ".repeat(15).trim())
        val wire = JSONObject().put("schema", AiConversationShareCodec.SCHEMA)
            .put("snapshot_id", shared.id).put("group_id", shared.groupId).put("title", shared.title)
            .put("summary", shared.summary).put("provider", shared.provider)
            .put("sender_name", shared.senderName).put("message_count", shared.messageCount)
        var opened: AiConversationShareCard? = null
        var openCount = 0
        val adapter = ChatAdapter(mutableListOf(ChatMessage("friend", AiConversationShareCodec.PREFIX + wire,
            id = "group-message", senderLabel = shared.senderName))).apply {
            onAiConversationShareOpen = { opened = it; openCount++ }
        }
        val holder = adapter.onCreateViewHolder(FrameLayout(activity), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)
        val body = requireNotNull(holder.attachmentList).getChildAt(0) as ViewGroup
        listOf(320, 360, 640).forEach { width ->
            measureRow(holder.itemView, width)
            val rectangle = bounds(holder.itemView as ViewGroup, body)
            assertTrue("card must be nonblank", rectangle.width() > 0 && rectangle.height() >= 48)
            assertTrue("card must fit measured row at $width px", rectangle.left >= 0 && rectangle.right <= width)
            assertChildrenFit(body)
        }
        assertEquals(View.GONE, holder.text.visibility)
        val labels = descendants(body).filterIsInstance<TextView>().map { it.text.toString() }
        assertTrue(labels.contains(shared.title))
        assertTrue(labels.contains(shared.summary))
        assertTrue(labels.any { it.contains(shared.senderName) && it.contains(shared.provider) })
        assertTrue(labels.contains(activity.getString(R.string.ai_conversation_share_message_count, shared.messageCount)))
        assertFalse(labels.any { it.contains("Viewer Only") })
        assertEquals(shared.senderName, descendants(body).filterIsInstance<TextView>()
            .single { it.contentDescription == shared.senderName }.contentDescription)
        body.performClick()
        assertEquals(1, openCount)
        assertEquals(shared, opened)
    }

    private fun withHost(test: (AiConversationShareReaderTestActivity) -> Unit) {
        val controller = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try { test(controller.get()) } finally { controller.pause().stop().destroy() }
    }

    private fun measureRow(view: View, width: Int) {
        view.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(4000, View.MeasureSpec.AT_MOST))
        view.layout(0, 0, width, view.measuredHeight)
    }

    private fun measurePage(view: View, width: Int, height: Int) {
        view.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(height, View.MeasureSpec.EXACTLY))
        view.layout(0, 0, width, height)
    }

    private fun bounds(root: ViewGroup, child: View) = Rect(0, 0, child.width, child.height).also {
        root.offsetDescendantRectToMyCoords(child, it)
    }

    private fun descendants(root: ViewGroup): List<View> = buildList {
        for (index in 0 until root.childCount) {
            val child = root.getChildAt(index)
            add(child)
            if (child is ViewGroup) addAll(descendants(child))
        }
    }

    private fun assertChildrenFit(parent: ViewGroup) {
        for (index in 0 until parent.childCount) {
            val child = parent.getChildAt(index)
            if (child.visibility == View.GONE) continue
            assertTrue("child stays inside card horizontally", child.left >= 0 && child.right <= parent.width)
            assertTrue("child stays inside card vertically", child.top >= 0 && child.bottom <= parent.height)
            if (child is ViewGroup) assertChildrenFit(child)
        }
    }

    private fun card() = AiConversationShareCard("ai_snapshot_layout", "group", "Shared title", "Excerpt", "chatgpt", "Shared Author", 2)
}

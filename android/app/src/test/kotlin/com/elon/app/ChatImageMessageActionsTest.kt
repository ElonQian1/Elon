package com.elon.app

import android.view.ContextThemeWrapper
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatImageMessageActionsTest {
    private fun context() = ContextThemeWrapper(RuntimeEnvironment.getApplication(), R.style.Theme_ElonApp)
    private fun imageMessage(id: String = "image") = ChatMessage("friend", "", id = id,
        attachments = listOf(ChatAttachment(kind = "image", displayName = "fixture.png")))

    @Test fun imageLongPressUsesMessageMenuAndRefreshesRecycledMessage() {
        val messages = mutableListOf(imageMessage("first"))
        var selected: ChatMessage? = null
        val adapter = ChatAdapter(messages, onMessageLongPress = { _, message -> selected = message })
        val holder = adapter.onCreateViewHolder(FrameLayout(context()), adapter.getItemViewType(0))
        adapter.onBindViewHolder(holder, 0)
        val image = holder.itemView.findViewById<ImageView>(R.id.chatMessageImage)
        assertTrue(image.performLongClick())
        assertSame(messages[0], selected)
        messages[0] = imageMessage("second")
        adapter.onBindViewHolder(holder, 0)
        assertSame(image, holder.itemView.findViewById(R.id.chatMessageImage))
        assertTrue(image.performLongClick())
        assertSame(messages[0], selected)
    }

    @Test fun ownedListenerIsRemovedButSpecializedVoiceListenerSurvives() {
        val host = LinearLayout(context())
        val image = ImageView(context())
        val voice = View(context())
        host.addView(image); host.addView(voice)
        var voiceCalls = 0
        voice.setOnLongClickListener { voiceCalls++; true }
        bindChatSelectionLongPress(host, View.OnLongClickListener { true })
        assertTrue(image.isLongClickable)
        bindChatSelectionLongPress(host, null)
        assertFalse(image.isLongClickable)
        assertTrue(voice.performLongClick())
        assertEquals(1, voiceCalls)
    }

    @Test fun tappingImageStillOpensFullSizeViewer() {
        val context = context()
        val file = java.io.File.createTempFile("tap-fixture", ".png", context.cacheDir)
        val bitmap = android.graphics.Bitmap.createBitmap(4, 4, android.graphics.Bitmap.Config.ARGB_8888)
        file.outputStream().use { bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
        bitmap.recycle()
        val host = LinearLayout(context)
        bindChatAttachmentViews(host, listOf(ChatAttachment(kind = "image", localPath = file.path)), messageActionsEnabled = true)
        var longPressed = false
        bindChatSelectionLongPress(host, View.OnLongClickListener { longPressed = true; true })
        assertTrue(host.findViewById<ImageView>(R.id.chatMessageImage).performClick())
        val dialog = org.robolectric.shadows.ShadowDialog.getLatestDialog()
        assertNotNull(dialog)
        assertTrue(dialog.isShowing)
        assertFalse(longPressed)
        dialog.dismiss()
    }

    @Test fun multiSelectExitRecreatesNormalImageActions() {
        val message = imageMessage()
        var calls = 0
        val adapter = ChatAdapter(mutableListOf(message), onMessageLongPress = { _, _ -> calls++ })
        val recycler = RecyclerView(context()).apply {
            layoutManager = LinearLayoutManager(context)
            this.adapter = adapter
        }
        fun layout() {
            recycler.measure(View.MeasureSpec.makeMeasureSpec(400, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(800, View.MeasureSpec.EXACTLY))
            recycler.layout(0, 0, 400, 800)
        }
        layout()
        adapter.startSelection(message)
        layout()
        val holder = recycler.findViewHolderForAdapterPosition(0) as ChatAdapter.VH
        val selectedImage = holder.itemView.findViewById<ImageView>(R.id.chatMessageImage)
        assertFalse(selectedImage.isLongClickable)
        assertTrue(selectedImage.performClick())
        assertEquals(0, adapter.selectedMessagesInOrder().size)
        adapter.exitSelection()
        layout()
        val image = recycler.findViewHolderForAdapterPosition(0)!!.itemView.findViewById<ImageView>(R.id.chatMessageImage)
        assertNotSame(selectedImage, image)
        assertTrue(image.performLongClick())
        assertEquals(1, calls)
    }

    @Test fun readerKeepsQrMenuAndChangingActionModeRebuildsAttachments() {
        val host = LinearLayout(context())
        val attachments = imageMessage().attachments
        bindChatAttachmentViews(host, attachments)
        val readerImage = host.findViewById<ImageView>(R.id.chatMessageImage)
        assertTrue(readerImage.isLongClickable)
        bindChatAttachmentViews(host, attachments, messageActionsEnabled = true)
        val chatImage = host.findViewById<ImageView>(R.id.chatMessageImage)
        assertNotSame(readerImage, chatImage)
        assertFalse(chatImage.isLongClickable)
    }

    @Test fun pureImageAndFileEnableContentActionsButRecalledAndEmptyDoNot() {
        for (attachment in listOf(ChatAttachment(kind = "image"), ChatAttachment(kind = "file"))) {
            val message = ChatMessage("friend", "", attachments = listOf(attachment))
            val policy = ChatMessageContentActions(message, "")
            assertTrue(policy.canCopy); assertTrue(policy.canForward); assertTrue(policy.canAnalyze)
            assertEquals(if (attachment.isImage()) 1 else 0, policy.images.size)
            message.recalledAt = "2026-09-29T00:00:00Z"
            val recalled = ChatMessageContentActions(message, "")
            assertFalse(recalled.canCopy); assertFalse(recalled.canForward); assertFalse(recalled.canAnalyze)
            assertTrue(recalled.images.isEmpty())
        }
        val empty = ChatMessageContentActions(ChatMessage("friend", ""), "")
        assertFalse(empty.canCopy); assertFalse(empty.canForward); assertFalse(empty.canAnalyze)
        assertTrue(ChatMessageContentActions(ChatMessage("friend", "text"), "text").canCopy)
    }
}

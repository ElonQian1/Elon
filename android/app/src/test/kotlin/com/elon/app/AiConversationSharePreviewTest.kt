package com.elon.app

import android.app.Application
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowDialog

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class AiConversationSharePreviewTest {
    @Test fun sendRequiresExplicitTargetAndConfirmationAndCanRetryWithoutLosingDraft() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        val draft = AiConversationShareDraft("chatgpt", "Original title", "Selected excerpt",
            listOf(ChatMessage("friend", "**selected**", id = "chatgpt_web:a")), emptySet())
        var sent = 0
        var previewed = 0
        var closed = 0
        var callback: ((AiConversationShareTarget) -> Unit)? = null
        val preview = AiConversationSharePreview(activity, draft, { previewed++ }, { callback = it },
            { group, title, summary, _ ->
                assertEquals("group1", group.id)
                assertEquals("Edited title", title)
                assertEquals("Selected excerpt", summary)
                sent++
            }, { closed++ })
        preview.show()
        shadowOf(Looper.getMainLooper()).idle()
        val dialog = ShadowDialog.getLatestDialog() as AlertDialog
        val root = dialog.window!!.decorView
        val send = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
        assertFalse(send.isEnabled)
        assertEquals(0, sent)
        val title = descendants(root).filterIsInstance<EditText>().first { it.hint == "卡片标题" }
        title.setText("Edited title")
        descendants(root).first { it.contentDescription == "ai-conversation-share-preview" }.performClick()
        assertEquals(1, previewed)
        descendants(root).first { it.contentDescription == "ai-conversation-share-target" }.performClick()
        assertEquals(0, sent)
        callback!!(AiConversationShareTarget("group1", "Selected group"))
        assertTrue(send.isEnabled)
        send.performClick()
        assertEquals(1, sent)
        preview.progress("Sending")
        assertFalse(send.isEnabled)
        assertFalse(title.isEnabled)
        assertFalse(dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled)
        preview.failed("Retry")
        assertTrue(send.isEnabled)
        assertEquals("Edited title", title.text.toString())
        var cancelled = false
        preview.progress("Preparing") { cancelled = true; preview.dismiss() }
        assertTrue(dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled)
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE).performClick()
        shadowOf(Looper.getMainLooper()).idle()
        assertTrue(cancelled)
        assertEquals(1, sent)
        assertEquals(1, closed)
        lifecycle.pause().stop().destroy()
    }

    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}

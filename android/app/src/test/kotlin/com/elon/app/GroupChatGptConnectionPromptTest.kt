package com.elon.app

import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
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
@Config(sdk = [34], qualifiers = "w360dp-h800dp-mdpi", application = android.app.Application::class)
class GroupChatGptConnectionPromptTest {
    @Test fun narrowRowKeepsModelWidthAndWrapsCopy() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        try {
            val prompt = GroupChatGptConnectionPrompt(activity) { true }
            val row = LinearLayout(activity)
            val model = TextView(activity).apply { text = "默认" }
            row.addView(model, LinearLayout.LayoutParams(142, 48).apply { marginEnd = 10 })
            prompt.attach(row, true)
            listOf(280, 320).forEach { width ->
                row.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                row.layout(0, 0, width, row.measuredHeight)
                assertEquals(142, model.width)
                assertTrue(prompt.root.right <= width)
                assertTrue(prompt.root.bottom <= row.height)
                for (index in 0 until prompt.root.childCount) {
                    val label = prompt.root.getChildAt(index) as TextView
                    assertTrue(label.bottom <= prompt.root.height)
                    assertEquals(0, label.layout.getEllipsisCount(label.lineCount - 1))
                }
            }
            assertTrue(prompt.root.contentDescription.contains("聊天不耗算力，又可训练群聊记忆"))
            prompt.close()
            assertNull(prompt.root.parent)
        } finally { lifecycle.pause().stop().destroy() }
    }

    @Test fun nonChatGptAndExpiredGroupCannotShowEntry() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        try {
            var valid = true
            val prompt = GroupChatGptConnectionPrompt(activity) { valid }
            val row = LinearLayout(activity)
            prompt.attach(row, false)
            assertEquals(View.GONE, prompt.root.visibility)
            prompt.attach(row, true)
            assertEquals(View.VISIBLE, prompt.root.visibility)
            valid = false
            prompt.resume()
            assertEquals(View.GONE, prompt.root.visibility)
            prompt.close()
        } finally { lifecycle.pause().stop().destroy() }
    }

    @Test fun loginRequiresExplicitActionAndCancelPreservesDraft() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        try {
            val draft = android.widget.EditText(activity).apply { setText("未发送的群消息") }
            activity.setContentView(draft)
            val prompt = GroupChatGptConnectionPrompt(activity) { true }
            prompt.attach(LinearLayout(activity), true)
            prompt.root.performClick()
            assertNull(shadowOf(activity).nextStartedActivity)
            ShadowDialog.getLatestDialog().cancel()
            assertEquals("未发送的群消息", draft.text.toString())
            assertNull(shadowOf(activity).nextStartedActivity)
            prompt.root.performClick()
            val dialog = ShadowDialog.getLatestDialog()
            dialog.window!!.decorView.findViewWithTag<View>("group-chatgpt-account-open").performClick()
            assertTrue(shadowOf(activity).nextStartedActivity.getBooleanExtra("chatgpt_return_after_login", false))
            assertEquals("未发送的群消息", draft.text.toString())
            prompt.close()
        } finally { lifecycle.pause().stop().destroy() }
    }
}

package com.elon.app

import android.app.Application
import android.widget.TextView
import android.widget.EditText
import android.os.Looper
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Robolectric
import org.robolectric.Shadows.shadowOf
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], manifest = Config.NONE, application = Application::class)
class GroupMentionTest {
    private val friend = GroupMentionTarget("u1", "张 三")
    private val ai = GroupMentionTarget("usr_elon_ai", "EL", isAi = true)

    @Test fun replacingTypedAtPreservesDraftAndMovesCursorAfterMention() {
        val edit = insertGroupMentions("你好 @请看看", 3, 4, listOf(friend))
        assertEquals("你好 @张 三 请看看", edit.text)
        assertEquals("你好 @张 三 ".length, edit.cursor)
    }

    @Test fun longPressAndMultiSelectKeepEmojiAndFollowingText() {
        val text = "😀早上好世界"
        val edit = insertGroupMentions(text, 5, 5, listOf(ai, friend, ai))
        assertEquals("😀早上好 @EL @张 三 世界", edit.text)
        assertEquals('世', edit.text[edit.cursor])
    }

    @Test fun selectionReplacementAndEqualNamesPreserveDistinctMembers() {
        val edit = insertGroupMentions("请替换这里。", 1, 5, listOf(friend, friend.copy(id = "u2")))
        assertEquals("请 @张 三 @张 三 。", edit.text)
    }

    @Test fun onlyNewAtAtWordBoundaryTriggersPicker() {
        assertTrue(isGroupMentionTrigger("@", 0, 0, 1))
        assertTrue(isGroupMentionTrigger("你好，＠", 3, 0, 1))
        assertFalse(isGroupMentionTrigger("user@", 4, 0, 1))
        assertFalse(isGroupMentionTrigger("@EL", 0, 0, 3))
        assertFalse(isGroupMentionTrigger("@", 0, 1, 1))
    }

    @Test fun searchFindsChineseNamesAndAiAndHandlesEmptyResults() {
        val members = listOf(ai, friend, GroupMentionTarget("u2", "Alice"))
        assertEquals(listOf(friend), filterGroupMentions(members, " @张 "))
        assertEquals(listOf(ai), filterGroupMentions(members, "群ai"))
        assertEquals("Alice", filterGroupMentions(members, "alice").single().name)
        assertTrue(filterGroupMentions(members, "未找到").isEmpty())
    }

    @Test fun recycledAvatarClearsMentionListenerForPrivateOrRecalledMessages() {
        val view = TextView(RuntimeEnvironment.getApplication())
        val message = ChatMessage("friend", "你好", senderLabel = "张三", senderUserId = "u1")
        var count = 0
        bindGroupMentionAvatar(view, message) { count++ }
        assertTrue(view.performLongClick())
        assertEquals(1, count)
        bindGroupMentionAvatar(view, message, null)
        assertFalse(view.isLongClickable)
        bindGroupMentionAvatar(view, message.copy(recalledAt = "now")) { count++ }
        assertFalse(view.isLongClickable)
        assertEquals(1, count)
    }

    @Test fun avatarMentionOpensComposerAndRestoresCursorAfterItsFocusHandler() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get()
        activity.setTheme(R.style.Theme_ElonApp)
        lifecycle.setup()
        val input = EditText(activity)
        activity.setContentView(input)
        input.setText("正文尾部")
        input.setSelection(2)
        var opened = false
        val mentions = GroupMentionController(activity, input, OkHttpClient(), "https://unused.invalid", { "self" }) {
            opened = true
            input.requestFocus()
            input.setSelection(input.text.length)
        }
        mentions.setGroup(AppGroup("g1", "测试群", 115, emptyList(), null, null, null, 0))
        mentions.mentionSender(ChatMessage("friend", "你好", senderLabel = "群友甲", senderUserId = "u1"))
        shadowOf(Looper.getMainLooper()).idle()
        assertTrue(opened)
        assertEquals("正文 @群友甲 尾部", input.text.toString())
        assertEquals('尾', input.text[input.selectionStart])
        mentions.setGroup(null)
        mentions.mentionSender(ChatMessage("friend", "你好", senderLabel = "其他人", senderUserId = "u2"))
        assertEquals("正文 @群友甲 尾部", input.text.toString())
        lifecycle.pause().stop().destroy()
    }
}

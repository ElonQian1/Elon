package com.elon.app.chatrecords

import android.app.Activity
import android.app.Application
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import com.elon.app.sociallinks.SocialLink
import com.elon.app.sociallinks.SocialLinkCardView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowPopupMenu

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordLinkActionsTest {
    @Test fun longPressKeepsCardClickAndCopiesExactUrl() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var clicks = 0
        val item = SocialLink("https://mp.weixin.qq.com/s?key=a%2Bb&scene=1#rd", "微信公众号", "示例文章")
        val card = SocialLinkCardView(activity) { clicks++ }.apply { bind(item) }
        val host = LinearLayout(activity).apply { addView(card) }; activity.setContentView(host)
        ChatRecordLinkActions.bind(host, card) { item }
        card.title.performClick(); assertEquals(1, clicks)
        assertTrue(card.title.performLongClick())
        val menu = ShadowPopupMenu.getLatestPopupMenu()
        assertEquals("转发", menu.menu.getItem(0).title); assertEquals("复制链接", menu.menu.getItem(1).title)
        menu.dismiss(); card.title.performClick(); assertEquals(2, clicks)
        fun children(view: View): List<View> = listOf(view) + if (view is ViewGroup) (0 until view.childCount).flatMap { children(view.getChildAt(it)) } else emptyList()
        children(host).filterIsInstance<Button>().single { it.text == "复制链接" }.performClick()
        assertEquals(item.url, (activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).primaryClip!!.getItemAt(0).text)
    }
}

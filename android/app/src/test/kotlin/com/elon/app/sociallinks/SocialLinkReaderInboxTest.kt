package com.elon.app.sociallinks

import android.app.Application
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import android.os.Looper

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class SocialLinkReaderInboxTest {
    private val app = RuntimeEnvironment.getApplication()
    private class Ui : SocialLinkReaderInbox.Ui {
        var count = -1; var latest: SocialLinkReaderInbox.Arrival? = null
        override fun onInbox(count: Int, latest: SocialLinkReaderInbox.Arrival?) { this.count = count; this.latest = latest }
    }
    private val ui = Ui()
    private fun arrival(id: String, group: String = "g1", sender: String = "张三") =
        SocialLinkReaderInbox.Arrival(SocialLinkReaderInbox.Conversation("group", group, "项目群"), sender, "在吗？", id)

    @After fun tearDown() { SocialLinkReaderInbox.unbind(ui); SocialLinkReaderInbox.clear() }

    @Test fun collectsOnlyOthersMessagesWhileAReaderIsBoundAndDeduplicatesIds() {
        SocialLinkReaderInbox.accept(arrival("m0"), "u2", "me")
        SocialLinkReaderInbox.bind(app, ui)
        assertEquals(0, ui.count)
        SocialLinkReaderInbox.accept(arrival("m1"), "me", "me")
        assertEquals(0, ui.count)
        SocialLinkReaderInbox.accept(arrival("m2"), "u2", "me")
        SocialLinkReaderInbox.accept(arrival("m2"), "u2", "me")
        SocialLinkReaderInbox.accept(arrival("m3", sender = "李四"), "u3", "me")
        assertEquals(2, ui.count)
        assertEquals("李四", ui.latest!!.sender)
        assertEquals("group:g1", SocialLinkReaderInbox.target()!!.key)
        SocialLinkReaderInbox.unbind(ui)
        SocialLinkReaderInbox.accept(arrival("m4"), "u2", "me")
        assertEquals(2, ui.count)
    }

    @Test fun wsEventsFromTheChatAreDeliveredOnTheMainThreadAndTheBannerClears() {
        SocialLinkReaderInbox.bind(app, ui)
        SocialLinkReaderInbox.onGlobalWsEvent(com.elon.app.GlobalWsEvent.FriendMessage("u9", "me", "f1", "【一龙文章】\n{\"schema\":1,\"article_id\":\"article_1\",\"revision\":1,\"title\":\"新文章\",\"summary\":\"\"}", "", "王五"))
        shadowOf(Looper.getMainLooper()).idle()
        assertEquals(1, ui.count)
        assertEquals("[文章] 新文章", ui.latest!!.preview)
        assertEquals("friend:u9", SocialLinkReaderInbox.target()!!.key)
        SocialLinkReaderInbox.clear()
        assertEquals(0, ui.count)
        assertNull(SocialLinkReaderInbox.target())
    }
}

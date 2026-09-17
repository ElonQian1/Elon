package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.view.View
import android.widget.FrameLayout
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class SocialLinkReaderSessionsTest {
    private val activity: Activity = Robolectric.buildActivity(Activity::class.java).setup().get()
    private val ui = object : SocialLinkReaderSessions.Ui {
        var loading: Boolean? = null
        override fun onStatus(text: String) {}
        override fun onLoading(loading: Boolean) { this.loading = loading }
        override fun onShowFullscreen(view: View, callback: android.webkit.WebChromeClient.CustomViewCallback) {}
        override fun onHideFullscreen() {}
    }
    private fun link(n: Int) = SocialLinkPolicy.link("https://mp.weixin.qq.com/s/session-$n")!!
    @After fun tearDown() { SocialLinkReaderSessions.destroyAll() }

    @Test fun minimisingKeepsThePageAliveAndReturningReattachesTheSameWebView() {
        val host = FrameLayout(activity)
        val session = SocialLinkReaderSessions.obtain(activity, link(1))
        SocialLinkReaderSessions.attach(session, activity, host, ui)
        val web = session.web
        assertSame(host, web.parent); assertFalse(session.minimized); assertEquals(true, ui.loading)
        SocialLinkReaderSessions.detach(session, activity.applicationContext)
        assertNull(web.parent); assertTrue(session.minimized)
        assertEquals(listOf(session), SocialLinkReaderSessions.minimized())
        assertSame(session, SocialLinkReaderSessions.obtain(activity, link(1)))
        val again = FrameLayout(activity)
        SocialLinkReaderSessions.attach(session, activity, again, ui)
        assertSame(web, session.web); assertSame(again, web.parent); assertFalse(session.minimized)
    }

    @Test fun oldestMinimisedSessionIsEvictedAtCapacityAndCloseDestroys() {
        val first = SocialLinkReaderSessions.obtain(activity, link(1)); SocialLinkReaderSessions.detach(first, activity)
        val second = SocialLinkReaderSessions.obtain(activity, link(2)); SocialLinkReaderSessions.detach(second, activity)
        val third = SocialLinkReaderSessions.obtain(activity, link(3))
        assertEquals(3, SocialLinkReaderSessions.all().size)
        val fourth = SocialLinkReaderSessions.obtain(activity, link(4))
        assertEquals(SocialLinkReaderSessions.MAX_SESSIONS, SocialLinkReaderSessions.all().size)
        assertFalse(SocialLinkReaderSessions.isLive(first)); assertTrue(SocialLinkReaderSessions.isLive(third)); assertTrue(SocialLinkReaderSessions.isLive(fourth))
        SocialLinkReaderSessions.destroy(fourth)
        assertFalse(SocialLinkReaderSessions.isLive(fourth)); assertEquals(2, SocialLinkReaderSessions.all().size)
    }

    @Test fun bubbleFollowsMinimisedSessionsAndReopensTheLatest() {
        val bubble = SocialLinkFloatingBubble.install(activity)
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertEquals(View.GONE, bubble.root.visibility)
        val a = SocialLinkReaderSessions.obtain(activity, link(1)); SocialLinkReaderSessions.detach(a, activity)
        val b = SocialLinkReaderSessions.obtain(activity, link(2)); SocialLinkReaderSessions.detach(b, activity)
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertEquals(View.VISIBLE, bubble.root.visibility)
        assertTrue(bubble.root.contentDescription.toString().contains("共 2 篇"))
        bubble.root.performClick()
        val started = shadowOf(activity).nextStartedActivity
        assertEquals(SocialLinkBrowserActivity::class.java.name, started.component?.className)
        assertEquals(link(2).url, started.getStringExtra("url"))
        SocialLinkReaderSessions.destroyAll()
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertEquals(View.GONE, bubble.root.visibility)
        bubble.dispose()
    }

    class ChatLikeActivity : Activity()
    class OtherActivity : Activity()

    @Test fun lifecycleInstallerAttachesBubbleToChatScreensOnly() {
        SocialLinkFloatingBubble.installForChat(activity.application, ChatLikeActivity::class.java)
        val chat = Robolectric.buildActivity(ChatLikeActivity::class.java).setup().get()
        assertNotNull((chat.window.decorView as android.view.ViewGroup).findViewWithTag<View>("social-link-bubble"))
        val other = Robolectric.buildActivity(OtherActivity::class.java).setup().get()
        assertNull((other.window.decorView as android.view.ViewGroup).findViewWithTag<View>("social-link-bubble"))
    }
}

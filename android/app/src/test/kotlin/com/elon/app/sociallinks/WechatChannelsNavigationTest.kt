package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.net.Uri
import android.view.View
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class WechatChannelsNavigationTest {
    private val source = "https://channels.weixin.qq.com/finder-preview/pages/sph?id=example"
    private val target = "weixin://biz/finder/openFinderFeed/exportId%3Dexport%2Fabcdefgh12345678%26actionType%3D0"
    @After fun cleanup() { SocialLinkReaderSessions.destroyAll() }

    @Test fun officialMainFrameRoutesToNativeConfirmationNotGenericBlock() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val session = SocialLinkReaderSessions.obtain(activity, SocialLinkPolicy.link(source)!!)
        var offered = 0
        session.ui = object : SocialLinkReaderSessions.Ui {
            override fun onStatus(text: String) {}
            override fun onLoading(loading: Boolean) {}
            override fun onShowFullscreen(view: View, callback: WebChromeClient.CustomViewCallback) {}
            override fun onHideFullscreen() {}
            override fun onWechatLink(url: String) { assertEquals(target, url); offered++ }
        }
        session.web.loadUrl(source)
        fun request(main: Boolean) = object : WebResourceRequest {
            override fun getUrl() = Uri.parse(target)
            override fun isForMainFrame() = main
            override fun isRedirect() = false
            override fun hasGesture() = false // Official handler awaits its scene request first.
            override fun getMethod() = "GET"
            override fun getRequestHeaders() = emptyMap<String, String>()
        }
        assertTrue(session.web.webViewClient.shouldOverrideUrlLoading(session.web, request(true)))
        assertEquals(1, offered)
        assertTrue(session.web.webViewClient.shouldOverrideUrlLoading(session.web, request(false)))
        session.web.loadUrl("https://example.com/")
        assertTrue(session.web.webViewClient.shouldOverrideUrlLoading(session.web, request(true)))
        assertEquals(1, offered)
    }

    @Test fun launchIsExplicitWechatWithoutForeignExtrasOrUriGrants() {
        val intent = WechatChannelsHandoff.intent(target)
        assertEquals("com.tencent.mm", intent.`package`)
        assertEquals(android.content.Intent.ACTION_VIEW, intent.action)
        assertTrue(intent.hasCategory(android.content.Intent.CATEGORY_BROWSABLE))
        assertNull(intent.component)
        assertNull(intent.extras)
        assertEquals(android.content.Intent.FLAG_ACTIVITY_NEW_TASK, intent.flags)
    }

    @Test fun portraitCardRetainsNativeTapAndFixedAspect() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var taps = 0
        val card = SocialLinkCardView(activity, true) { taps++ }
        card.bind(SocialLinkPolicy.link("https://weixin.qq.com/sph/example")!!.copy(author = "Creator", title = "Video"))
        card.performClick()
        assertEquals(1, taps)
        assertTrue(card.contentDescription.startsWith("在微信打开"))
        card.measure(View.MeasureSpec.makeMeasureSpec(220, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
        card.layout(0, 0, 220, card.measuredHeight)
        val media = card.getChildAt(0)
        val footer = card.getChildAt(card.childCount - 1)
        assertEquals("The cover retains its 3:4 aspect independently of the author footer", 293, media.measuredHeight)
        assertTrue(footer.height >= (48 * card.resources.displayMetrics.density).toInt())
        assertEquals(media.bottom, footer.top)
        assertEquals(footer.bottom, card.height)
        assertTrue(WechatChannelsPolicy.sameContent("https://weixin.qq.com/sph/example", source))
        assertFalse(WechatChannelsPolicy.sameContent("https://weixin.qq.com/sph/other", source))
    }
}

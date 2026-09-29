package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.graphics.Bitmap
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.bindChatSelectionContent
import com.elon.app.bindChatSelectionLongPress
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.time.Duration

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class SocialLinkCardInteractionTest {
    private fun screen(child: View): FrameLayout {
        val activity = child.context as Activity
        val root = FrameLayout(activity); root.addView(child, FrameLayout.LayoutParams(280, -2))
        activity.setContentView(root)
        root.measure(View.MeasureSpec.makeMeasureSpec(390, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(844, View.MeasureSpec.EXACTLY))
        root.layout(0, 0, 390, 844)
        return root
    }
    private fun touch(root: View, target: View, long: Boolean = false) {
        assertTrue("Touch target is laid out", target.width > 0 && target.height > 0)
        val position = IntArray(2); val origin = IntArray(2)
        target.getLocationOnScreen(position); root.getLocationOnScreen(origin)
        val x = position[0] - origin[0] + target.width / 2f; val y = position[1] - origin[1] + target.height / 2f
        val now = SystemClock.uptimeMillis()
        root.dispatchTouchEvent(MotionEvent.obtain(now, now, MotionEvent.ACTION_DOWN, x, y, 0))
        if (long) shadowOf(android.os.Looper.getMainLooper()).idleFor(Duration.ofMillis(700))
        root.dispatchTouchEvent(MotionEvent.obtain(now, SystemClock.uptimeMillis(), MotionEvent.ACTION_UP, x, y, 0))
        shadowOf(android.os.Looper.getMainLooper()).idle()
    }
    @Test fun legacyLongPressChildConsumesTheCardTap() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var opened = 0
        val parent = LinearLayout(activity).apply { setOnClickListener { opened++ } }
        val child = TextView(activity).apply { text = "文章标题"; minHeight = 60 }
        parent.addView(child); bindChatSelectionLongPress(parent, View.OnLongClickListener { true })
        touch(screen(parent), child)
        assertEquals("Regression reproducer: child consumes tap without opening parent", 0, opened)
    }
    @Test fun titleSourceCoverAndPaddingOpenAndLongPressStillWorks() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var opened = 0; var longPressed = 0
        val card = SocialLinkCardView(activity) { opened++ }
        card.bind(SocialLink("https://mp.weixin.qq.com/s/test", "微信公众号", "文章标题", "公众号"))
        card.cover.setImageBitmap(Bitmap.createBitmap(2, 2, Bitmap.Config.ARGB_8888)); card.cover.visibility = View.VISIBLE
        val root = screen(card)
        bindChatSelectionLongPress(root, View.OnLongClickListener { longPressed++; true })
        for (target in listOf(card.title, card.source, card.cover)) touch(root, target)
        val now = SystemClock.uptimeMillis()
        root.dispatchTouchEvent(MotionEvent.obtain(now, now, MotionEvent.ACTION_DOWN, 1f, 1f, 0))
        root.dispatchTouchEvent(MotionEvent.obtain(now, now + 50, MotionEvent.ACTION_UP, 1f, 1f, 0))
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertEquals(4, opened)
        touch(root, card.title, true)
        assertEquals(4, opened); assertEquals(1, longPressed)
    }
    @Test fun selectionReplacesNavigationAndRebindRestoresIt() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var opened = 0; var selected = 0
        fun card() = SocialLinkCardView(activity) { opened++ }.apply { bind(SocialLink("https://v.douyin.com/test", "抖音", "视频")) }
        val first = card(); val root = screen(first)
        bindChatSelectionContent(root, View.OnClickListener { selected++ })
        touch(root, first.title); assertEquals(1, selected); assertEquals(0, opened)
        val next = card(); val rebound = screen(next)
        bindChatSelectionLongPress(rebound, View.OnLongClickListener { true })
        touch(rebound, next.title); assertEquals(1, opened)
    }
    @Test fun fallbackBadgeAndTimestampRemainClickableWithChatLongPress() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var opened = 0; var held = 0
        val card = SocialLinkCardView(activity) { opened++ }
        card.bind(SocialLinkPolicy.link("https://www.bilibili.com/video/BV1enYL6SEtU/?t=80")!!)
        val root = screen(card)
        bindChatSelectionLongPress(root, View.OnLongClickListener { held++; true })
        assertEquals("B站", card.badge.text); assertEquals("从 01:20 开始", card.time.text)
        assertEquals(View.GONE, card.cover.visibility)
        touch(root, card.badge); touch(root, card.time)
        assertEquals(2, opened)
        touch(root, card.badge, true); assertEquals(1, held); assertEquals(2, opened)
    }
    @Test fun readerUsesExplicitActivityAndPreservesOriginalUrl() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val item = SocialLink("https://mp.weixin.qq.com/s/test?from=groupmessage", "微信公众号")
        SocialLinkBrowserActivity.open(activity, item)
        val intent = shadowOf(activity).nextStartedActivity
        assertEquals(SocialLinkBrowserActivity::class.java.name, intent.component?.className)
        assertEquals(item.url, intent.getStringExtra("url"))
    }
    @Test fun videoChromeIsVisibleBeforeCoverAndDoesNotBreakSelection() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var opened = 0; var selected = 0; var held = 0
        val item = SocialLinkPolicy.link("https://weixin.qq.com/sph/example")!!.copy(author = "Long creator name", authorAvatar = "first")
        val card = SocialLinkCardView(activity, true) { opened++ }
        card.bind(item)
        val root = screen(card)
        assertEquals(View.VISIBLE, card.play.visibility); assertEquals(View.GONE, card.cover.visibility)
        assertEquals("视频号", card.badge.text); assertEquals("视频号 · ${item.author}", card.source.text)
        assertTrue(card.source.bottom <= card.height); assertTrue(card.play.width > 0)
        card.bindAvatar(Bitmap.createBitmap(2, 2, Bitmap.Config.ARGB_8888))
        assertEquals(View.VISIBLE, card.creatorAvatar.visibility)
        bindChatSelectionLongPress(root, View.OnLongClickListener { held++; true })
        for (target in listOf(card.play, card.badge, card.source, card.creatorAvatar)) touch(root, target)
        assertEquals(4, opened)
        touch(root, card.play, true); assertEquals(1, held); assertEquals(4, opened)
        bindChatSelectionContent(root, View.OnClickListener { selected++ })
        touch(root, card.play); assertEquals(1, selected); assertEquals(4, opened)
        card.bind(item.copy(authorAvatar = null))
        assertEquals(View.GONE, card.creatorAvatar.visibility)
    }
    @Test fun mediaProvidersUseLargeCoverAndKeepNavigationAndSelection() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        for (url in listOf("https://www.bilibili.com/video/BV19eYH6NEsC/?t=80", "https://v.douyin.com/example/", "https://www.xiaohongshu.com/discovery/item/example")) {
            val item = SocialLinkPolicy.link(url)!!.copy(title = "Media title", image = "https://example.org/cover.jpg")
            assertTrue(SocialLinkPresentation.mediaCard(item))
            var opened = 0; var selected = 0
            val card = SocialLinkCardView(activity, true) { opened++ }
            card.bind(item); card.bindCover(Bitmap.createBitmap(160, 90, Bitmap.Config.ARGB_8888))
            val root = screen(card)
            assertEquals(View.VISIBLE, card.title.visibility)
            assertEquals(280, card.cover.width)
            assertEquals(157, card.cover.height)
            bindChatSelectionLongPress(root, View.OnLongClickListener { true })
            touch(root, card.play); touch(root, card.title); assertEquals(2, opened)
            bindChatSelectionContent(root, View.OnClickListener { selected++ })
            touch(root, card.play); assertEquals(1, selected); assertEquals(2, opened)
            card.bind(item.copy(image = null)); assertEquals(View.GONE, card.cover.visibility)
        }
        for (url in listOf("https://www.bilibili.com/", "https://www.xiaohongshu.com/", "https://v.douyin.com.evil.test/a")) {
            assertFalse(SocialLinkPresentation.mediaCard(SocialLinkPolicy.link(url)!!))
        }
    }
}

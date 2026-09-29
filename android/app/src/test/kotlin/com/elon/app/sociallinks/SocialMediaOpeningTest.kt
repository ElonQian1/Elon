package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.pm.ActivityInfo
import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.ResolveInfo
import android.view.View
import android.widget.LinearLayout
import com.elon.app.AuthManager
import com.elon.app.bindChatSelectionContent
import com.elon.app.bindChatSelectionLongPress
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class, qualifiers = "mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class SocialMediaOpeningTest {
    private lateinit var activity: Activity
    private val note = "https://www.xiaohongshu.com/explore/0123456789abcdef01234567?xsec_token=fixture%2Btoken&xsec_source=pc_share"
    @Before fun setup() {
        activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        activity.getSharedPreferences("social-media-opening-v1", Context.MODE_PRIVATE).edit().clear().commit()
        AuthManager.prefs(activity).edit().putString("auth_user_id", "synthetic-a").commit()
    }
    private fun install(platform: SocialMediaPlatform) {
        shadowOf(activity.packageManager).installPackage(PackageInfo().apply {
            packageName = platform.packageName
            applicationInfo = ApplicationInfo().apply { packageName = platform.packageName; enabled = true }
        })
    }
    private fun bar(url: String = note, opened: (SocialMediaOpenMode) -> Unit = {}): SocialMediaOpenBar {
        val link = SocialLinkPolicy.link(url)!!
        return SocialMediaOpenBar(activity, SocialMediaOpenPolicy.platform(link)!!, { link }, opened)
    }
    @Test fun providerIsDerivedFromHostAndPreservesFullShareParameters() {
        assertEquals(SocialMediaPlatform.XIAOHONGSHU, SocialMediaOpenPolicy.platform(SocialLink(note, "抖音")))
        assertNull(SocialMediaOpenPolicy.platform(SocialLink("https://www.xiaohongshu.com.evil.test/explore/a", "小红书")))
        assertNull(SocialMediaOpenPolicy.platform(SocialLink("https://user@www.xiaohongshu.com/explore/a", "小红书")))
        assertEquals(listOf(note), SocialMediaOpenPolicy.candidates(SocialLinkPolicy.link(note)!!))
        val bili = "https://www.bilibili.com/video/BV19eYH6NEsC/?t=80&p=2"
        assertEquals(listOf(bili), SocialMediaOpenPolicy.candidates(SocialLinkPolicy.link(bili)!!))
        val dy = "https://www.douyin.com/video/12345678?share=keep%2Bquery"
        assertEquals("https://www.iesdouyin.com/share/video/12345678/?share=keep%2Bquery", SocialMediaOpenPolicy.candidates(SocialLinkPolicy.link(dy)!!).first())
    }
    @Test fun shortRedirectsCannotEscapeProviderOrDropSignedParameters() {
        val short = SocialLinkPolicy.link("https://xhslink.com/a/fixture")!!
        assertEquals(note, SocialMediaOpenPolicy.redirect(short, short.url, note))
        for (bad in listOf("http://www.xiaohongshu.com/explore/a", "https://evil.test/a", "https://127.0.0.1/a", "intent://item/a", "https://www.douyin.com/video/12345678")) {
            assertNull(SocialMediaOpenPolicy.redirect(short, short.url, bad))
        }
        assertEquals(note, SocialMediaOpenPolicy.destination(short, note)?.url)
        assertNull(SocialMediaOpenPolicy.destination(short, "https://www.xiaohongshu.com/login"))
        assertNull(SocialMediaOpenPolicy.redirect(SocialLinkPolicy.link(note)!!, note, note.replace("0123456789abcdef01234567", "000000000000000000000000")))
    }
    @Test fun preferencesArePerPlatformPerAccountAndSurviveRecreation() {
        val store = SocialMediaOpenPreferences(activity)
        assertEquals(SocialMediaOpenMode.APP, store.get(SocialMediaPlatform.XIAOHONGSHU))
        store.set(SocialMediaPlatform.XIAOHONGSHU, SocialMediaOpenMode.READER)
        assertEquals(SocialMediaOpenMode.READER, SocialMediaOpenPreferences(activity).get(SocialMediaPlatform.XIAOHONGSHU))
        assertEquals(SocialMediaOpenMode.APP, store.get(SocialMediaPlatform.DOUYIN))
        AuthManager.prefs(activity).edit().putString("auth_user_id", "synthetic-b").commit()
        assertFalse(store.current())
        store.set(SocialMediaPlatform.XIAOHONGSHU, SocialMediaOpenMode.APP)
        assertEquals(SocialMediaOpenMode.APP, SocialMediaOpenPreferences(activity).get(SocialMediaPlatform.XIAOHONGSHU))
        AuthManager.prefs(activity).edit().putString("auth_user_id", "synthetic-a").commit()
        assertEquals(SocialMediaOpenMode.READER, store.get(SocialMediaPlatform.XIAOHONGSHU))
        assertTrue(store.preferences.all.values.all { it == "APP" || it == "READER" })
    }
    @Test fun missingAppFallsBackWithoutChangingPreferenceOrOpeningStore() {
        val view = View(activity)
        val item = SocialLinkPolicy.link(note)!!
        SocialMediaCardAction.open(view, item)
        val intent = shadowOf(activity).nextStartedActivity
        assertEquals(SocialLinkBrowserActivity::class.java.name, intent.component?.className)
        assertEquals(note, intent.getStringExtra("url"))
        assertEquals(SocialMediaOpenMode.APP, SocialMediaOpenPreferences(activity).get(SocialMediaPlatform.XIAOHONGSHU))
        val bar = bar()
        assertFalse(bar.appButton.isEnabled); assertTrue(bar.appButton.text.contains("未安装"))
        assertTrue(bar.readerButton.text.contains("默认"))
    }
    @Test fun installedAppWithNoContentHandlerStillFallsBackToReader() {
        install(SocialMediaPlatform.XIAOHONGSHU)
        SocialMediaCardAction.open(View(activity), SocialLinkPolicy.link(note)!!)
        assertEquals(SocialLinkBrowserActivity::class.java.name, shadowOf(activity).nextStartedActivity.component?.className)
    }
    @Test fun validContentIntentTargetsOnlyProviderPackageAndReaderChoiceNeverLaunchesApp() {
        val platform = SocialMediaPlatform.XIAOHONGSHU
        install(platform)
        val request = SocialMediaAppLauncher.intent(platform, note)
        shadowOf(activity.packageManager).addResolveInfoForIntent(request, ResolveInfo().apply {
            activityInfo = ActivityInfo().apply { packageName = platform.packageName; name = "Router"; exported = true; enabled = true }
        })
        assertTrue(SocialMediaAppLauncher.open(activity, SocialLinkPolicy.link(note)!!))
        val sent = shadowOf(activity).nextStartedActivity
        assertEquals(platform.packageName, sent.`package`); assertEquals(note, sent.dataString)
        assertEquals(0, sent.flags); assertNull(sent.extras)
        SocialMediaCardAction.open(View(activity), SocialLinkPolicy.link(note)!!, SocialMediaOpenMode.READER)
        assertEquals(SocialLinkBrowserActivity::class.java.name, shadowOf(activity).nextStartedActivity.component?.className)
    }
    @Test fun footerChangesDefaultAndOpensWithoutReorderingAndSelectionStillWins() {
        install(SocialMediaPlatform.XIAOHONGSHU)
        val opened = mutableListOf<SocialMediaOpenMode>()
        val bar = bar(opened = { opened += it })
        val parent = LinearLayout(activity).apply { addView(bar) }; activity.setContentView(parent)
        bar.readerButton.performClick()
        assertEquals(SocialMediaOpenMode.READER, SocialMediaOpenPreferences(activity).get(SocialMediaPlatform.XIAOHONGSHU))
        assertTrue(bar.readerButton.text.contains("默认"))
        bar.appButton.performClick()
        assertEquals(listOf(SocialMediaOpenMode.READER, SocialMediaOpenMode.APP), opened)
        assertSame(bar.appButton, bar.getChildAt(0)); assertSame(bar.readerButton, bar.getChildAt(1))
        var held = 0; var selected = 0
        bindChatSelectionLongPress(parent, View.OnLongClickListener { held++; true })
        bar.appButton.performLongClick(); assertEquals(1, held); assertEquals(2, opened.size)
        bindChatSelectionContent(parent, View.OnClickListener { selected++ })
        bar.readerButton.performClick(); assertEquals(1, selected); assertEquals(2, opened.size)
        assertEquals(SocialMediaOpenMode.APP, SocialMediaOpenPreferences(activity).get(SocialMediaPlatform.XIAOHONGSHU))
    }
    @Test fun allPlatformsShareFooterWithAccessibleTargetsAtLargeFontAndNarrowWidths() {
        for (platform in SocialMediaPlatform.entries) install(platform)
        for (url in listOf(note, "https://weixin.qq.com/sph/example", "https://v.douyin.com/example/", "https://b23.tv/example")) {
            for (size in listOf(14f, 21f, 28f)) {
                val bar = bar(url)
                bar.appButton.textSize = size; bar.readerButton.textSize = size
                bar.measure(View.MeasureSpec.makeMeasureSpec(220, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                bar.layout(0, 0, 220, bar.measuredHeight)
                for (button in listOf(bar.appButton, bar.readerButton).filter { it.visibility == View.VISIBLE }) {
                    assertTrue(button.height >= 48); assertTrue(button.width >= 48)
                    val bounds = "$url size=$size button=${button.left},${button.top},${button.right},${button.bottom} bar=${bar.width}x${bar.height}"
                    assertTrue(bounds, button.bottom <= bar.height); assertTrue(bounds, button.right <= bar.width)
                    assertTrue(button.layout.height <= button.height - button.paddingTop - button.paddingBottom)
                }
                if (size >= 21f && bar.readerButton.visibility == View.VISIBLE) assertEquals(LinearLayout.VERTICAL, bar.orientation)
            }
        }
    }
    @Test fun wrappedActivityPreservesCallerTaskButApplicationContextRequiresNewTask() {
        val platform = SocialMediaPlatform.XIAOHONGSHU
        install(platform)
        shadowOf(activity.packageManager).addResolveInfoForIntent(SocialMediaAppLauncher.intent(platform, note), ResolveInfo().apply {
            activityInfo = ActivityInfo().apply { packageName = platform.packageName; name = "Router"; exported = true; enabled = true }
        })
        assertTrue(SocialMediaAppLauncher.open(ContextWrapper(activity), SocialLinkPolicy.link(note)!!))
        assertEquals(0, shadowOf(activity).nextStartedActivity.flags)
        assertTrue(SocialMediaAppLauncher.open(activity.applicationContext, SocialLinkPolicy.link(note)!!))
        assertEquals(Intent.FLAG_ACTIVITY_NEW_TASK, shadowOf(activity.application as Application).nextStartedActivity.flags)
        assertEquals(Intent.FLAG_ACTIVITY_NEW_TASK, WechatChannelsHandoff.intent("weixin://fixture").flags)
    }
    @Test fun wechatHidesReaderIgnoresOldDefaultAndDoesNotFallBackIntoWebView() {
        val platform = SocialMediaPlatform.WECHAT
        val url = "https://weixin.qq.com/sph/example"
        SocialMediaOpenPreferences(activity).set(platform, SocialMediaOpenMode.READER)
        val opened = mutableListOf<SocialMediaOpenMode>()
        val bar = bar(url) { opened += it }
        assertEquals(View.GONE, bar.readerButton.visibility)
        bar.readerButton.performClick()
        assertTrue(opened.isEmpty())
        assertEquals(SocialMediaOpenMode.APP, SocialMediaOpenPolicy.requestedMode(platform, SocialMediaOpenMode.READER))
        SocialMediaCardAction.open(View(activity), SocialLinkPolicy.link(url)!!)
        assertNull(shadowOf(activity).nextStartedActivity)
        assertTrue(org.robolectric.shadows.ShadowToast.getTextOfLatestToast().contains("微信"))
    }
}

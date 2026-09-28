package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.content.Intent
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class WechatChannelsTaskTest {
    @Test fun externalWechatCannotBackgroundTheChatTask() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val target = "weixin://biz/finder/openFinderFeed/exportId%3Dexample"
        activity.startActivity(WechatChannelsHandoff.intent(target))
        val launched = shadowOf(activity).nextStartedActivity

        // WeChat's intermediate Activity calls moveTaskToBack on video exit.
        assertEquals(Intent.FLAG_ACTIVITY_NEW_TASK, launched.flags)
        assertEquals("com.tencent.mm", launched.`package`)
        assertEquals(Intent.ACTION_VIEW, launched.action)
        assertEquals(target, launched.dataString)
        assertTrue(launched.hasCategory(Intent.CATEGORY_BROWSABLE))
        assertFalse(activity.isFinishing)
    }

    @Test fun internalReaderStillReturnsWithinTheOriginalTask() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        SocialLinkBrowserActivity.open(activity, SocialLink("https://weixin.qq.com/sph/example", "Video"))
        val launched = shadowOf(activity).nextStartedActivity
        assertEquals(SocialLinkBrowserActivity::class.java.name, launched.component?.className)
        assertEquals(0, launched.flags)
        assertFalse(activity.isFinishing)
    }
}

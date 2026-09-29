package com.elon.app

import android.view.View
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class MainSystemBarChromeTest {
    @Test fun systemBarIconsFollowTheirActualThemeSurface() {
        Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup().use { host ->
            val activity = host.get()
            for (night in listOf(false, true, false)) {
                @Suppress("DEPRECATION")
                activity.resources.updateConfiguration(android.content.res.Configuration(activity.resources.configuration).apply {
                    uiMode = (uiMode and android.content.res.Configuration.UI_MODE_NIGHT_MASK.inv()) or
                        if (night) android.content.res.Configuration.UI_MODE_NIGHT_YES else android.content.res.Configuration.UI_MODE_NIGHT_NO
                }, activity.resources.displayMetrics)
                applyMainSystemBarChrome(activity, null)
                val flags = activity.window.decorView.systemUiVisibility
                assertEquals(!night, flags and View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR != 0)
                assertEquals(!night, flags and View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR != 0)
                assertEquals(activity.getColor(R.color.mobile_surface), activity.window.statusBarColor)
            }
        }
    }

    @Test
    fun homeAndChatChromeBothLeaveStatusBarLayoutToTheSystem() {
        listOf(false, true).forEach { drawChatBehindNavigationBar ->
            val flags = resolveMainSystemUiVisibility(
                currentFlags = View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or View.SYSTEM_UI_FLAG_FULLSCREEN,
                drawChatBehindNavigationBar = drawChatBehindNavigationBar,
                sdkInt = 34
            )

            assertEquals(0, flags and View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN)
            assertEquals(0, flags and View.SYSTEM_UI_FLAG_FULLSCREEN)
        }
    }

    @Test
    fun toolbarAddsOnlyTheMissingStatusBarInset() {
        assertEquals(0, resolveMainToolbarTopMargin(statusBarInsetTop = 135, rootTopInWindow = 135))
        assertEquals(135, resolveMainToolbarTopMargin(statusBarInsetTop = 135, rootTopInWindow = 0))
        assertEquals(0, resolveMainToolbarTopMargin(statusBarInsetTop = 135, rootTopInWindow = 160))
    }
}

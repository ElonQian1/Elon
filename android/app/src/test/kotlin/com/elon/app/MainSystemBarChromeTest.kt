package com.elon.app

import android.view.View
import org.junit.Assert.assertEquals
import org.junit.Test

class MainSystemBarChromeTest {
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

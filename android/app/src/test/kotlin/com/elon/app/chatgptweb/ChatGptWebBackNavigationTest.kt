package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebBackNavigationTest {
    @Test
    fun dismissesOfficialOverlayBeforeUsingWebHistory() {
        assertEquals(
            ChatGptWebBackNavigation.Action.DISMISS_OFFICIAL_OVERLAY,
            ChatGptWebBackNavigation.decide(
                manifestWithRegion(ChatGptWebUiRegion.OVERLAY),
                canGoBack = true,
                officialViewActive = true,
            ),
        )
    }

    @Test
    fun usesWebHistoryWhenNoOfficialOverlayIsVisible() {
        assertEquals(
            ChatGptWebBackNavigation.Action.NAVIGATE_WEB_HISTORY,
            ChatGptWebBackNavigation.decide(
                manifestWithRegion(ChatGptWebUiRegion.HEADER),
                canGoBack = true,
                officialViewActive = true,
            ),
        )
    }

    @Test
    fun exitsOfficialViewWhenItsOverlayAndHistoryAreExhausted() {
        assertEquals(
            ChatGptWebBackNavigation.Action.EXIT_OFFICIAL_VIEW,
            ChatGptWebBackNavigation.decide(null, canGoBack = false, officialViewActive = true),
        )
    }

    @Test
    fun finishesOnlyWhenNoOfficialSurfaceOrWebHistoryRemains() {
        assertEquals(
            ChatGptWebBackNavigation.Action.FINISH_ACTIVITY,
            ChatGptWebBackNavigation.decide(null, canGoBack = false, officialViewActive = false),
        )
    }

    private fun manifestWithRegion(region: String) = ChatGptWebUiManifest(
        version = 3,
        pageKind = "conversation",
        title = "Example",
        compatibility = "healthy",
        controls = listOf(
            ChatGptWebUiControl(
                id = "control_example",
                semantic = "more",
                label = "More",
                region = region,
                role = "button",
                enabled = true,
                selected = false,
            ),
        ),
    )
}

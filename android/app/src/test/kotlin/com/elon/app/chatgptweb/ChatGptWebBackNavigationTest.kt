package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebBackNavigationTest {
    @Test
    fun dismissesOfficialOverlayBeforeUsingWebHistory() {
        assertEquals(
            ChatGptWebBackNavigation.Action.DISMISS_OFFICIAL_OVERLAY,
            ChatGptWebBackNavigation.decide(manifestWithRegion(ChatGptWebUiRegion.OVERLAY), true),
        )
    }

    @Test
    fun usesWebHistoryWhenNoOfficialOverlayIsVisible() {
        assertEquals(
            ChatGptWebBackNavigation.Action.NAVIGATE_WEB_HISTORY,
            ChatGptWebBackNavigation.decide(manifestWithRegion(ChatGptWebUiRegion.HEADER), true),
        )
    }

    @Test
    fun finishesOnlyWhenNoOverlayOrWebHistoryRemains() {
        assertEquals(
            ChatGptWebBackNavigation.Action.FINISH_ACTIVITY,
            ChatGptWebBackNavigation.decide(null, false),
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

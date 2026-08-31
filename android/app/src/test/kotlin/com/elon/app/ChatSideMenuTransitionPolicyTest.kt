package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatSideMenuTransitionPolicyTest {
    @Test
    fun animatedHandoffRunsAfterTheDrawerCloseAnimation() {
        assertTrue(
            ChatSideMenuTransitionPolicy.closeHandoffDelayMs(animated = true) >
                ChatSideMenuTransitionPolicy.ANIMATION_DURATION_MS,
        )
    }

    @Test
    fun immediateCloseDoesNotDelayTheNextSurface() {
        assertEquals(0L, ChatSideMenuTransitionPolicy.closeHandoffDelayMs(animated = false))
    }
}

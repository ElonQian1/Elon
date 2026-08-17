package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatSendFallbackPolicyTest {
    @Test
    fun loadingAnonymousProviderRetriesWithoutOpeningTheLegacyLoginSurface() {
        assertEquals(
            WebChatSendFallbackPolicy.Action.RETRY_IN_PLACE,
            WebChatSendFallbackPolicy.decide(loginRequired = false),
        )
    }

    @Test
    fun explicitAuthenticationPageRetriesGuestAccessBeforeOfferingLogin() {
        assertEquals(
            WebChatSendFallbackPolicy.Action.RETRY_GUEST_ACCESS,
            WebChatSendFallbackPolicy.decide(loginRequired = true),
        )
    }
}

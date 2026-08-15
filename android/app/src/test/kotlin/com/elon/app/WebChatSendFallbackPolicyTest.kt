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
    fun explicitAuthenticationPageCanOpenTheOfficialAuthenticationSurface() {
        assertEquals(
            WebChatSendFallbackPolicy.Action.OPEN_OFFICIAL_AUTHENTICATION,
            WebChatSendFallbackPolicy.decide(loginRequired = true),
        )
    }
}

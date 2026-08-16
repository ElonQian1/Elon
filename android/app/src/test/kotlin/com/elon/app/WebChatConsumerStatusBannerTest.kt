package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConsumerStatusBannerTest {
    private val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)

    @Test
    fun anonymousReadyChatDoesNotShowARecoveryBanner() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "ready")

        assertFalse(state.visible)
        assertFalse(state.retryVisible)
        assertFalse(state.officialVisible)
    }

    @Test
    fun connectionFailureOffersRetryWithoutDiscardingTheNativeComposer() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "error")

        assertTrue(state.visible)
        assertTrue(state.retryVisible)
        assertTrue(state.officialVisible)
        assertTrue(state.message.contains(chatGpt.displayName))
    }

    @Test
    fun explicitLoginRequirementOffersGuestRetryAndOptionalOfficialLogin() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "login_required")

        assertTrue(state.visible)
        assertTrue(state.retryVisible)
        assertTrue(state.officialVisible)
        assertEquals("可尝试免费访客聊天，或登录账号", state.message)
        assertEquals("访客", state.retryLabel)
        assertEquals("登录", state.officialLabel)
    }
}

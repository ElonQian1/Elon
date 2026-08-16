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
    fun explicitLoginRequirementOffersOfficialCompletionButNotFakeRetry() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "login_required")

        assertTrue(state.visible)
        assertFalse(state.retryVisible)
        assertTrue(state.officialVisible)
        assertEquals("当前网页需要登录", state.message)
    }
}

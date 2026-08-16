package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProviderDraftStateTest {
    @Test
    fun keepsIndependentProviderDraftsAndRemovesBlankValues() {
        val state = WebChatProviderDraftState()

        state.remember(WebChatProviderId.CHATGPT_WEB, "ChatGPT draft")
        state.remember(WebChatProviderId.GOOGLE_WEB, "Google draft")

        assertEquals("ChatGPT draft", state.restore(WebChatProviderId.CHATGPT_WEB))
        assertEquals("Google draft", state.restore(WebChatProviderId.GOOGLE_WEB))

        state.remember(WebChatProviderId.GOOGLE_WEB, "   ")
        assertEquals("", state.restore(WebChatProviderId.GOOGLE_WEB))
    }
}

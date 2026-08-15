package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

class WebChatProductionSelectorsTest {
    @Test
    fun providerScopedComposerSelectorsStayDistinct() {
        val chatGpt = WebChatProductionSelectors.composerInput(WebChatProviderId.CHATGPT_WEB)
        val google = WebChatProductionSelectors.composerInput(WebChatProviderId.GOOGLE_WEB)

        assertEquals("web-chat-composer-input:chatgpt_web", chatGpt)
        assertEquals("web-chat-composer-input:google_web", google)
        assertNotEquals(chatGpt, google)
        assertEquals(
            "web-chat-attachment:chatgpt_web",
            WebChatProductionSelectors.attachment(WebChatProviderId.CHATGPT_WEB),
        )
    }

    @Test
    fun actionSelectorsDoNotChangeWithVisualState() {
        assertEquals(
            WebChatProductionSelectors.SEND,
            WebChatProductionSelectors.composerAction(streaming = false),
        )
        assertEquals(
            WebChatProductionSelectors.STOP_GENERATION,
            WebChatProductionSelectors.composerAction(streaming = true),
        )
    }

    @Test
    fun providerMenusUseTheSameProductionNamespace() {
        assertEquals(
            "web-chat-composer-tools:chatgpt_web",
            WebChatProductionSelectors.composerTools(WebChatProviderId.CHATGPT_WEB),
        )
        assertEquals(
            "web-chat-page-actions:google_web",
            WebChatProductionSelectors.pageActions(WebChatProviderId.GOOGLE_WEB),
        )
    }
}

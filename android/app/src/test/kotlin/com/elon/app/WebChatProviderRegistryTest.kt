package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProviderRegistryTest {
    @Test
    fun keepsWorkAndChatAsOrthogonalInteractionModes() {
        assertEquals(SocialAiInteractionMode.WORK, SocialAiInteractionMode.fromWireValue("work"))
        assertEquals(SocialAiInteractionMode.CHAT, SocialAiInteractionMode.fromWireValue("chat"))
        assertEquals(SocialAiInteractionMode.CHAT, SocialAiInteractionMode.fromWireValue("unknown"))
        assertEquals(SocialAiInteractionMode.CHAT, SocialAiInteractionMode.fromWireValue(null))
        assertEquals(null, SocialAiInteractionMode.parse("unknown"))
    }

    @Test
    fun exposesOnlyImplementedWebChatProviders() {
        val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val google = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)

        assertEquals("ChatGPT 网页 AI", chatGpt.displayName)
        assertTrue(chatGpt.available)
        assertTrue(chatGpt.selectable)
        assertTrue(chatGpt.supports(WebChatProviderCapability.MESSAGE_COPY))
        assertTrue(chatGpt.supports(WebChatProviderCapability.MESSAGE_REGENERATE))
        assertTrue(chatGpt.supports(WebChatProviderCapability.MESSAGE_CONTEXT_ACTIONS))
        assertTrue(chatGpt.supports(WebChatProviderCapability.MODEL_SELECTOR))
        assertTrue(chatGpt.supports(WebChatProviderCapability.ATTACHMENT_UPLOAD))
        assertTrue(chatGpt.supports(WebChatProviderCapability.COMPOSER_TOOLS))
        assertTrue(chatGpt.supports(WebChatProviderCapability.STOP_GENERATION))
        assertTrue(chatGpt.supports(WebChatProviderCapability.DICTATION))
        assertTrue(chatGpt.supports(WebChatProviderCapability.REALTIME_VOICE))
        assertTrue(chatGpt.supports(WebChatProviderCapability.RICH_TEXT))
        assertTrue(chatGpt.supports(WebChatProviderCapability.RICH_PARTS))
        assertEquals("Google 搜索网页 AI", google.displayName)
        assertEquals(R.drawable.ic_web_ai_google_avatar, google.avatarResId)
        assertTrue(google.available)
        assertTrue(google.selectable)
        assertTrue(google.supports(WebChatProviderCapability.MESSAGE_COPY))
        assertFalse(google.supports(WebChatProviderCapability.MESSAGE_REGENERATE))
        assertFalse(google.supports(WebChatProviderCapability.MESSAGE_CONTEXT_ACTIONS))
        assertFalse(google.supports(WebChatProviderCapability.COMPOSER_TOOLS))
        assertTrue(google.supports(WebChatProviderCapability.STOP_GENERATION))
        assertFalse(google.supports(WebChatProviderCapability.DICTATION))
        assertFalse(google.supports(WebChatProviderCapability.REALTIME_VOICE))
        assertTrue(google.supports(WebChatProviderCapability.RICH_TEXT))
        assertFalse(google.supports(WebChatProviderCapability.RICH_PARTS))
        assertEquals(
            listOf(WebChatProviderId.CHATGPT_WEB, WebChatProviderId.GOOGLE_WEB),
            WebChatProviderRegistry.available().map { it.id },
        )
    }

    @Test
    fun providerCannotBecomeSelectableWithoutNativeConversationAndProjectNavigation() {
        val incomplete = WebChatProviderIdentity(
            id = WebChatProviderId.GOOGLE_WEB,
            displayName = "Google 搜索网页 AI",
            avatarResId = 0,
            available = true,
            capabilities = setOf(WebChatProviderCapability.CONVERSATION_LIST),
        )

        assertFalse(incomplete.selectable)
    }
}

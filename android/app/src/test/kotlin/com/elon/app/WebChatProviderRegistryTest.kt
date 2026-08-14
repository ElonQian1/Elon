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
        assertEquals(WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION, chatGpt.capabilities)
        assertEquals("Google 搜索网页 AI", google.displayName)
        assertFalse(google.available)
        assertFalse(google.selectable)
        assertTrue(google.capabilities.isEmpty())
        assertEquals(listOf(WebChatProviderId.CHATGPT_WEB), WebChatProviderRegistry.available().map { it.id })
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

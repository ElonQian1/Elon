package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProviderPickerTest {
    @Test
    fun exposesBothProvidersAndShowsOnlyTheCurrentProviderModel() {
        val options = webChatProviderPickerOptions(
            providers = WebChatProviderRegistry.available(),
            selectedProvider = WebChatProviderId.CHATGPT_WEB,
            currentModel = "GPT-5 Fast",
        )

        assertEquals(listOf(WebChatProviderId.CHATGPT_WEB, WebChatProviderId.GOOGLE_WEB), options.map { it.providerId })
        assertTrue(options.first().selected)
        assertTrue(options.first().label.contains("GPT-5 Fast"))
        assertFalse(options.last().selected)
        assertFalse(options.last().label.contains("GPT-5 Fast"))
    }

    @Test
    fun selectedGoogleProviderKeepsItsObservedModelLabel() {
        val option = webChatProviderPickerOptions(
            providers = WebChatProviderRegistry.available(),
            selectedProvider = WebChatProviderId.GOOGLE_WEB,
            currentModel = "Google AI 模式",
        ).last()

        assertTrue(option.selected)
        assertEquals("Google 搜索网页 AI · Google AI 模式", option.label)
    }
}

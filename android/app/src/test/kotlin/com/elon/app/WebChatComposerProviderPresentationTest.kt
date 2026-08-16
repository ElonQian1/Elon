package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatComposerProviderPresentationTest {
    @Test
    fun descriptionKeepsProviderAndObservedModelDistinct() {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val description = WebChatComposerProviderPresentation.description(provider, "GPT-5 Fast")

        assertEquals("聊天模式；提供方：ChatGPT 网页 AI；模型：GPT-5 Fast", description)
    }

    @Test
    fun googlePresentationDoesNotClaimAChatGptModel() {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)
        val description = WebChatComposerProviderPresentation.description(provider, "Google AI 模式")

        assertTrue(description.contains("Google 搜索网页 AI"))
        assertTrue(description.endsWith("Google AI 模式"))
    }
}

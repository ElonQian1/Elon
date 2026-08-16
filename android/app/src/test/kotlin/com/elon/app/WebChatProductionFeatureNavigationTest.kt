package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionFeatureNavigationTest {
    @Test
    fun parsesFeatureNavigationForTheProductionFriendChat() {
        val result = WebChatProductionFeatureParser.parse(listOf(
            feature("projects", "项目", "projects", selected = true, sensitive = false),
            feature("health", "健康", "health", selected = false, sensitive = true),
        ))

        assertEquals(listOf("projects", "health"), result.map { it.id })
        assertTrue(result.first().selected)
        assertTrue(result.last().requiresUserConfirmation)
        assertTrue(result.all(WebChatProductionFeature::officialCompletion))
        assertEquals("项目（当前·官网）", result.first().navigationLabel())
        assertEquals("健康（官网）", result.last().navigationLabel())
        assertEquals("chatgpt-feature:projects", result.first().nativeSelector)
    }

    @Test
    fun ignoresInvalidAndDuplicateFeaturesAndUsesAStableFallbackSelector() {
        val result = WebChatProductionFeatureParser.parse(listOf(
            feature("tasks", "任务", "tasks", false, false, selector = ""),
            feature("tasks", "重复任务", "tasks", true, false),
            feature("missing_label", "", "tasks", false, false),
        ))

        assertEquals(1, result.size)
        assertEquals("web-chat-feature:tasks", result.single().nativeSelector)
        assertFalse(result.single().selected)
    }

    @Test
    fun featureNavigationIsNativeForChatGptAndFallsBackForGoogle() {
        assertTrue(WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
            .supports(WebChatProviderCapability.FEATURE_NAVIGATION))
        assertFalse(WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)
            .supports(WebChatProviderCapability.FEATURE_NAVIGATION))
    }

    @Test
    fun returnsEmptyWhenThePageHasNoFeatureManifest() {
        assertTrue(WebChatProductionFeatureParser.parse(emptyList()).isEmpty())
    }

    private fun feature(
        id: String,
        label: String,
        kind: String,
        selected: Boolean,
        sensitive: Boolean,
        selector: String = "chatgpt-feature:$id",
    ) = WebChatConsumerFeature(
        id = id,
        label = label,
        kind = kind,
        selected = selected,
        requiresUserConfirmation = sensitive,
        nativeSelector = selector,
    )
}

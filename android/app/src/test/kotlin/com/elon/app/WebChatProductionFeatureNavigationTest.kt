package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionFeatureNavigationTest {
    @Test
    fun parsesFeatureNavigationForTheProductionFriendChat() {
        val navigation = JSONObject().put("features", JSONArray()
            .put(feature("projects", "项目", "projects", selected = true, sensitive = false))
            .put(feature("health", "健康", "health", selected = false, sensitive = true)))

        val result = WebChatProductionFeatureParser.parse(navigation)

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
        val navigation = JSONObject().put("features", JSONArray()
            .put(feature("tasks", "任务", "tasks", false, false, selector = ""))
            .put(feature("tasks", "重复任务", "tasks", true, false))
            .put(JSONObject().put("id", "missing_label")))

        val result = WebChatProductionFeatureParser.parse(navigation)

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
        assertTrue(WebChatProductionFeatureParser.parse(JSONObject()).isEmpty())
    }

    private fun feature(
        id: String,
        label: String,
        kind: String,
        selected: Boolean,
        sensitive: Boolean,
        selector: String = "chatgpt-feature:$id",
    ): JSONObject = JSONObject()
        .put("id", id)
        .put("label", label)
        .put("kind", kind)
        .put("selected", selected)
        .put("requires_user_confirmation", sensitive)
        .put("native_adb_content_description", selector)
}

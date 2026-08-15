package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionPageActionsTest {
    @Test
    fun parsesSupportedCurrentPageActionsForTheProductionChat() {
        val response = JSONObject().put("controls", JSONArray()
            .put(control("temporary", "临时聊天", "temporary_chat", "header", "direct"))
            .put(control("options", "会话操作", "conversation_options", "header", "direct"))
            .put(control("copy", "复制", "copy", "message", "dedicated")))

        val result = WebChatProductionPageActionParser.parse(response)

        assertEquals(listOf("temporary", "options"), result.map { it.controlId })
        assertFalse(result.first().officialFallback)
        assertEquals("selector:temporary", result.first().nativeSelector)
    }

    @Test
    fun keepsComplexMutationsVisibleButRoutesThemToTheOfficialFallback() {
        val response = JSONObject().put("controls", JSONArray()
            .put(control("rename", "重命名会话", "rename", "overlay", "menu"))
            .put(control("delete", "删除", "delete", "overlay", "menu", confirmation = true)))

        val result = WebChatProductionPageActionParser.parse(response)

        assertTrue(result.all(WebChatProductionPageAction::officialFallback))
        assertTrue(result.last().requiresUserConfirmation)
    }

    @Test
    fun keepsNewManifestActionsButIgnoresDisabledStructuralAndComposerControls() {
        val response = JSONObject().put("controls", JSONArray()
            .put(control("profile", "账户", "profile", "header", "direct"))
            .put(control("profile", "重复账户", "profile", "header", "direct"))
            .put(control("future", "新官网功能", "future_action", "content", "direct"))
            .put(control("navigation", "打开导航", "navigation", "header", "direct"))
            .put(control("model", "模型", "model", "composer", "dedicated"))
            .put(control("off", "停用", "more", "header", "direct", enabled = false)))

        val result = WebChatProductionPageActionParser.parse(response)

        assertEquals(listOf("profile", "future"), result.map { it.controlId })
    }

    @Test
    fun pageActionsAreNativeForChatGptAndUseOfficialGoogleFallback() {
        assertTrue(WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
            .supports(WebChatProviderCapability.PAGE_ACTIONS))
        assertFalse(WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)
            .supports(WebChatProviderCapability.PAGE_ACTIONS))
    }

    private fun control(
        id: String,
        label: String,
        semantic: String,
        region: String,
        presentation: String,
        confirmation: Boolean = false,
        enabled: Boolean = true,
    ): JSONObject = JSONObject()
        .put("control_id", id)
        .put("label", label)
        .put("semantic", semantic)
        .put("region", region)
        .put("enabled", enabled)
        .put("requires_user_confirmation", confirmation)
        .put("native_presentation", presentation)
        .put("native_adb_content_description", "selector:$id")
}

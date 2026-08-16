package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionPageActionsTest {
    @Test
    fun parsesSupportedCurrentPageActionsForTheProductionChat() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control("temporary", "临时聊天", "temporary_chat", "header", WebChatConsumerControlPresentation.DIRECT),
            control("options", "会话操作", "conversation_options", "header", WebChatConsumerControlPresentation.DIRECT),
            control("copy", "复制", "copy", "message", WebChatConsumerControlPresentation.DEDICATED),
        ))

        assertEquals(listOf("temporary", "options"), result.map { it.controlId })
        assertFalse(result.first().officialFallback)
        assertEquals("selector:temporary", result.first().nativeSelector)
    }

    @Test
    fun keepsAdaptiveMutationsNativeAndRoutesExternalFlowsToTheOfficialFallback() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control("rename", "重命名会话", "rename", "overlay", WebChatConsumerControlPresentation.MENU),
            control("delete", "删除", "delete", "overlay", WebChatConsumerControlPresentation.MENU, confirmation = true),
            control("share", "分享", "share", "overlay", WebChatConsumerControlPresentation.MENU),
        ))

        assertFalse(result[0].officialFallback)
        assertFalse(result[1].officialFallback)
        assertTrue(result[1].requiresUserConfirmation)
        assertTrue(result[2].officialFallback)
    }

    @Test
    fun keepsNewManifestActionsButIgnoresDisabledStructuralAndComposerControls() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control("profile", "账户", "profile", "header", WebChatConsumerControlPresentation.DIRECT),
            control("profile", "重复账户", "profile", "header", WebChatConsumerControlPresentation.DIRECT),
            control("future", "新官网功能", "future_action", "content", WebChatConsumerControlPresentation.DIRECT),
            control("navigation", "打开导航", "navigation", "header", WebChatConsumerControlPresentation.DIRECT),
            control("model", "模型", "model", "composer", WebChatConsumerControlPresentation.DEDICATED),
            control("off", "停用", "more", "header", WebChatConsumerControlPresentation.DIRECT, enabled = false),
        ))

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
        presentation: WebChatConsumerControlPresentation,
        confirmation: Boolean = false,
        enabled: Boolean = true,
    ) = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = id,
            label = label,
            semantic = semantic,
            region = region,
            role = "button",
            enabled = enabled,
            selected = false,
        ),
        requiresUserConfirmation = confirmation,
        presentation = presentation,
        nativeSelector = "selector:$id",
    )
}

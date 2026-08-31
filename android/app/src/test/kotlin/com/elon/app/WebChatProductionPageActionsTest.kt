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
            control(
                "options", "打开当前聊天选项", "conversation_options", "header",
                WebChatConsumerControlPresentation.DIRECT, contextId = "current",
            ),
            control("copy", "复制", "copy", "message", WebChatConsumerControlPresentation.DEDICATED),
        ), conversationIdentity)

        assertEquals(listOf("options"), result.map { it.controlId })
        assertEquals(listOf("会话设置"), result.map { it.label })
        assertFalse(result.first().officialFallback)
        assertEquals("selector:options", result.first().nativeSelector)
    }

    @Test
    fun keepsAdaptiveMutationsNativeAndRoutesExternalFlowsToTheOfficialFallback() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control("rename", "重命名会话", "rename", "overlay", WebChatConsumerControlPresentation.MENU),
            control("pin", "取消置顶", "pin", "overlay", WebChatConsumerControlPresentation.MENU, confirmation = true),
            control("archive", "Unarchive", "archive", "overlay", WebChatConsumerControlPresentation.MENU, confirmation = true),
            control("delete", "删除", "delete", "overlay", WebChatConsumerControlPresentation.MENU, confirmation = true),
            control("share", "分享", "share", "overlay", WebChatConsumerControlPresentation.MENU),
        ), conversationIdentity)

        val bySemantic = result.associateBy(WebChatProductionPageAction::semantic)
        assertFalse(bySemantic.getValue("rename").officialFallback)
        assertEquals("取消置顶", bySemantic.getValue("pin").label)
        assertEquals("取消归档", bySemantic.getValue("archive").label)
        assertTrue(bySemantic.getValue("pin").requiresUserConfirmation)
        assertTrue(bySemantic.getValue("archive").requiresUserConfirmation)
        assertFalse(bySemantic.getValue("delete").officialFallback)
        assertTrue(bySemantic.getValue("delete").requiresUserConfirmation)
        assertTrue(bySemantic.getValue("share").officialFallback)
    }

    @Test
    fun onlyExplicitPageActionsReachTheMenuAndUnknownDomControlsStayInTheOfficialFallback() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control(
                "profile", "账户", "profile", "header", WebChatConsumerControlPresentation.DIRECT,
                placement = WebChatConsumerPageActionPlacement.PAGE,
            ),
            control(
                "profile-copy", "重复账户", "profile", "header", WebChatConsumerControlPresentation.DIRECT,
                placement = WebChatConsumerPageActionPlacement.PAGE,
            ),
            control(
                "future", "新官网功能", "future_action", "content", WebChatConsumerControlPresentation.DIRECT,
                placement = WebChatConsumerPageActionPlacement.NONE,
            ),
            control("navigation", "打开导航", "navigation", "header", WebChatConsumerControlPresentation.DIRECT),
            control("model", "模型", "model", "composer", WebChatConsumerControlPresentation.DEDICATED),
            control("off", "停用", "more", "header", WebChatConsumerControlPresentation.DIRECT, enabled = false),
        ), pageIdentity)

        assertEquals(listOf("profile"), result.map { it.controlId })
        assertEquals("账号与设置", result.single().label)
    }

    @Test
    fun excludesConversationListRowsAndOptionsBelongingToAnotherConversation() {
        val result = WebChatProductionPageActionParser.parse(listOf(
            control(
                "current-options", "打开当前会话选项", "conversation_options", "header",
                WebChatConsumerControlPresentation.DIRECT, contextId = "current",
            ),
            control(
                "other-options", "打开制冰机选购指南的对话选项", "conversation_options", "header",
                WebChatConsumerControlPresentation.DIRECT, contextId = "other",
            ),
            control(
                "conversation-row", "制冰机选购指南 制冰机坏了", "action", "content",
                WebChatConsumerControlPresentation.MENU,
                placement = WebChatConsumerPageActionPlacement.NONE,
            ),
        ), conversationIdentity)

        assertEquals(listOf("current-options"), result.map { it.controlId })
    }

    @Test
    fun recognizesProjectConversations() {
        val projectConversation = WebChatProductionPageIdentity.from(state(
            pageKind = "conversation",
            pageUrl = "https://chatgpt.com/g/g-p-project/c/conversation-id",
        ))

        assertEquals("conversation-id", projectConversation.conversationId)
        assertTrue(projectConversation.hasConversationTarget)
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
        contextId: String? = null,
        placement: WebChatConsumerPageActionPlacement =
            WebChatConsumerPageActionPlacement.CONVERSATION,
    ) = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = id,
            label = label,
            semantic = semantic,
            region = region,
            role = "button",
            enabled = enabled,
            selected = false,
            contextId = contextId,
        ),
        requiresUserConfirmation = confirmation,
        presentation = presentation,
        nativeSelector = "selector:$id",
        pageActionPlacement = placement,
    )

    private val conversationIdentity = WebChatProductionPageIdentity(
        pageKind = "conversation",
        path = "/c/current",
        conversationId = "current",
    )
    private val pageIdentity = WebChatProductionPageIdentity(
        pageKind = "feature",
        path = "/settings",
        conversationId = null,
    )

    private fun state(pageKind: String, pageUrl: String) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = pageKind,
        pageUrl = pageUrl,
        features = emptyList(),
        controls = emptyList(),
        commandRequests = emptyList(),
    )
}

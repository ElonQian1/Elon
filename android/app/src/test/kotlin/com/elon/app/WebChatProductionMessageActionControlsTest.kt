package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionMessageActionControlsTest {
    @Test
    fun discoversOnlyEnabledNonCopyMessageContexts() {
        val controls = listOf(
            control("copy", "复制", "message-1", enabled = true),
            control("read_aloud", "朗读", "message-1", enabled = true),
            control("action", "不可用", "message-2", enabled = false),
            control("action", "页面操作", "", enabled = true, region = "content"),
        )

        assertEquals(
            setOf("message-1"),
            WebChatProductionMessageActionControls.messageContextIds(controls),
        )
    }

    @Test
    fun exposesSafeNativeContextActionsWithoutDuplicatingCopyOrFallbackControls() {
        val controls = listOf(
            control("copy", "复制", "message-1", enabled = true),
            control(
                semantic = "read_aloud",
                label = "朗读",
                contextId = "message-1",
                enabled = true,
                controlId = "control_read_aloud",
                confirmation = true,
            ),
            control(
                semantic = "action",
                label = "官网专用",
                contextId = "message-1",
                enabled = true,
                controlId = "control_official",
                presentation = WebChatConsumerControlPresentation.OFFICIAL_FALLBACK,
            ),
            control("action", "其他消息", "message-2", enabled = true),
        )

        assertEquals(
            listOf(
                WebChatContextAction(
                    "control_read_aloud",
                    "朗读",
                    true,
                    "selector:control_read_aloud",
                ),
            ),
            WebChatProductionMessageActionControls.contextActions(controls, "message-1"),
        )
    }

    @Test
    fun consumerActionsExplainSuccessfulImmediateResults() {
        assertEquals("已复制消息", WebChatProductionMessageActionFeedback.copyAccepted())
        assertEquals("正在重新生成回答…", WebChatProductionMessageActionFeedback.regenerateAccepted())
        assertEquals(
            "已执行：朗读",
            WebChatProductionMessageActionFeedback.contextActionAccepted("朗读"),
        )
    }

    private fun control(
        semantic: String,
        label: String,
        contextId: String,
        enabled: Boolean,
        controlId: String = "control_${semantic}_$contextId",
        region: String = "message",
        confirmation: Boolean = false,
        presentation: WebChatConsumerControlPresentation = WebChatConsumerControlPresentation.MENU,
    ) = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = controlId,
            semantic = semantic,
            label = label,
            region = region,
            role = "button",
            enabled = enabled,
            selected = false,
            contextId = contextId,
        ),
        requiresUserConfirmation = confirmation,
        presentation = presentation,
        nativeSelector = "selector:$controlId",
    )
}

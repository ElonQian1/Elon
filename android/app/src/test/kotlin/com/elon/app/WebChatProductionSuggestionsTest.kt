package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionSuggestionsTest {
    @Test
    fun keepsOnlyDirectEnabledSuggestionControlsForTheProductionStrip() {
        val result = WebChatProductionSuggestionParser.parse(listOf(
            control("first", "总结这段对话", "suggestion"),
            control("project", "打开项目", "project", confirmation = true),
            control("menu", "更多建议", "suggestion", presentation = WebChatConsumerControlPresentation.MENU),
            control("header", "标题操作", "more", region = "header"),
            control("disabled", "不可用", "suggestion", enabled = false),
        ))

        assertEquals(listOf("first", "project"), result.map(WebChatProductionSuggestion::controlId))
        assertFalse(result.first().requiresUserConfirmation)
        assertTrue(result.last().requiresUserConfirmation)
    }

    @Test
    fun deduplicatesAndCapsTheInlineStripWhileOverflowRemainsInPageActions() {
        val result = WebChatProductionSuggestionParser.parse(
            (1..6).map { index -> control("item$index", "建议 $index", "suggestion") } +
                control("item1", "重复建议", "suggestion"),
        )

        assertEquals(listOf("item1", "item2", "item3", "item4"), result.map { it.controlId })
    }

    @Test
    fun exposesStableSelectorsForConsumerAndAutomationUse() {
        assertEquals("web-chat-suggestions", WebChatProductionSelectors.SUGGESTIONS)
        assertEquals(
            "web-chat-suggestion:chatgpt_web:control_suggestion_1",
            WebChatProductionSelectors.suggestion(
                WebChatProviderId.CHATGPT_WEB,
                "control suggestion/1",
            ),
        )
    }

    private fun control(
        id: String,
        label: String,
        semantic: String,
        region: String = "suggestions",
        presentation: WebChatConsumerControlPresentation = WebChatConsumerControlPresentation.DIRECT,
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

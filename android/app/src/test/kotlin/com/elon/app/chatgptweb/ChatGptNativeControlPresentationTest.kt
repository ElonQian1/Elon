package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptNativeControlPresentationTest {
    @Test
    fun keepsSecondaryCopyActionsInsteadOfCollapsingAllCopyControls() {
        val controls = listOf(
            control("copy_reply", "copy", "复制回复", ChatGptWebUiRegion.MESSAGE, CONTEXT),
            control("copy_table", "copy", "复制表格", ChatGptWebUiRegion.MESSAGE, CONTEXT),
            control("copy_code", "copy", "复制", ChatGptWebUiRegion.MESSAGE, CONTEXT),
            control("share", "share", "分享", ChatGptWebUiRegion.MESSAGE, CONTEXT),
        )

        val actions = ChatGptNativeControlPresentation.messageActions(controls).getValue(CONTEXT)
        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(listOf("复制表格", "复制", "分享"), actions.map(ChatGptWebUiControl::label))
        assertEquals("dedicated", coverage.getValue("copy_reply").kind.wireName)
        assertEquals(
            "chatgpt-message-copy:$CONTEXT",
            coverage.getValue("copy_reply").nativeSelector,
        )
        assertEquals("menu", coverage.getValue("copy_table").kind.wireName)
        assertEquals(
            "chatgpt-message-actions:$CONTEXT:3",
            coverage.getValue("copy_table").nativeTriggerSelector,
        )
    }

    @Test
    fun exposesTwoHeaderActionsAndReportsOverflowAsOfficialFallback() {
        val controls = listOf(
            control("more", "more", "更多", ChatGptWebUiRegion.HEADER),
            control("sources", "sources", "文件和来源", ChatGptWebUiRegion.HEADER),
            control("future", "settings", "未来动作", ChatGptWebUiRegion.HEADER),
        )

        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(listOf("more", "sources"),
            ChatGptNativeControlPresentation.headerActions(controls).map(ChatGptWebUiControl::id))
        assertEquals("direct", coverage.getValue("sources").kind.wireName)
        assertEquals(true, ChatGptNativeControlPresentation.usesHeaderIcon(controls[1]))
        assertEquals(false, ChatGptNativeControlPresentation.usesHeaderIcon(controls[0]))
        assertEquals("official_fallback", coverage.getValue("future").kind.wireName)
        assertNull(coverage.getValue("future").nativeSelector)
    }

    @Test
    fun givesOverlayEntriesStableNativeSelectorsAndTrigger() {
        val controls = listOf(
            control("timestamp", "timestamp", "今天，4:40", ChatGptWebUiRegion.OVERLAY),
            control("sources", "sources", "查看来源", ChatGptWebUiRegion.OVERLAY),
            control("read", "read_aloud", "朗读", ChatGptWebUiRegion.OVERLAY),
        )

        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals("metadata", coverage.getValue("timestamp").kind.wireName)
        assertEquals("menu", coverage.getValue("sources").kind.wireName)
        assertEquals("chatgpt-control:sources:查看来源", coverage.getValue("sources").nativeSelector)
        assertEquals("chatgpt-overlay-actions:2", coverage.getValue("sources").nativeTriggerSelector)
    }

    private fun control(
        id: String,
        semantic: String,
        label: String,
        region: String,
        contextId: String? = null,
    ) = ChatGptWebUiControl(
        id = id,
        semantic = semantic,
        label = label,
        region = region,
        role = "button",
        enabled = true,
        selected = false,
        contextId = contextId,
    )

    private companion object {
        const val CONTEXT = "conversation-turn-2"
    }
}

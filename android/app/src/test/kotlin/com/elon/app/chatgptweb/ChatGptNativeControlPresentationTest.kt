package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptNativeControlPresentationTest {
    @Test
    fun treatsTheRenderedHeaderTitleAsMetadataInsteadOfAnOfficialFallback() {
        val title = control("title", "title", "工作", ChatGptWebUiRegion.HEADER)

        val coverage = ChatGptNativeControlPresentation.describe(listOf(title)).getValue("title")

        assertEquals("metadata", coverage.kind.wireName)
        assertNull(coverage.nativeSelector)
        assertNull(coverage.nativeTriggerSelector)
    }

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
            "chatgpt-message-actions:$CONTEXT",
            coverage.getValue("copy_table").nativeTriggerSelector,
        )
        assertEquals(
            "chatgpt-message-part:$CONTEXT:2:chart",
            ChatGptNativeControlPresentation.messagePartSelector(CONTEXT, 2, "chart"),
        )
    }

    @Test
    fun exposesTwoHeaderActionsAndMovesOverflowIntoTheNativeMenu() {
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
        assertEquals("menu", coverage.getValue("future").kind.wireName)
        assertEquals("chatgpt-control:future:未来动作", coverage.getValue("future").nativeSelector)
        assertEquals(
            "chatgpt-overflow-actions:1",
            coverage.getValue("future").nativeTriggerSelector,
        )
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

    @Test
    fun bindsMessageOverlayEntriesToTheTriggeringMessageSelector() {
        val controls = listOf(
            control("sources", "sources", "查看来源", ChatGptWebUiRegion.OVERLAY, CONTEXT),
            control("read", "read_aloud", "朗读", ChatGptWebUiRegion.OVERLAY, CONTEXT),
        )

        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(
            "chatgpt-message-overlay-actions:$CONTEXT",
            coverage.getValue("sources").nativeTriggerSelector,
        )
        assertEquals(
            "chatgpt-message-overlay-actions:$CONTEXT",
            coverage.getValue("read").nativeTriggerSelector,
        )
    }

    @Test
    fun exposesFeaturePageContentThroughTheNativePageActionsMenu() {
        val controls = listOf(
            control("create", "create_asset", "创建图片", ChatGptWebUiRegion.CONTENT),
            control("settings", "settings", "偏好设置", ChatGptWebUiRegion.CONTENT),
        )

        val actions = ChatGptNativeControlPresentation.pageActions(controls)
        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(listOf("create", "settings"), actions.map(ChatGptWebUiControl::id))
        assertEquals("menu", coverage.getValue("create").kind.wireName)
        assertEquals("chatgpt-control:create:创建图片", coverage.getValue("create").nativeSelector)
        assertEquals("chatgpt-page-actions:2", coverage.getValue("create").nativeTriggerSelector)
    }

    @Test
    fun unsupportedAriaSliderUsesTheOfficialPageInsteadOfABlindNativeTap() {
        val slider = ChatGptWebUiControl(
            id = "effort",
            semantic = "slider",
            label = "思考强度",
            region = ChatGptWebUiRegion.CONTENT,
            role = "slider",
            enabled = true,
            selected = false,
            inputKind = "range",
        )

        val coverage = ChatGptNativeControlPresentation.describe(listOf(slider)).getValue("effort")

        assertTrue(ChatGptNativeControlPresentation.pageActions(listOf(slider)).isEmpty())
        assertEquals("official_fallback", coverage.kind.wireName)
        assertTrue(ChatGptNativeControlPresentation.isExpectedOfficialFallback(slider))
    }

    @Test
    fun coversEveryBoundedFeatureControlIncludingDisabledEntries() {
        val controls = (1..48).map { index ->
            control(
                id = "feature_$index",
                semantic = "action",
                label = "功能 $index",
                region = ChatGptWebUiRegion.CONTENT,
                enabled = index != 48,
            )
        }

        val actions = ChatGptNativeControlPresentation.pageActions(controls)
        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(48, actions.size)
        assertEquals(false, actions.last().enabled)
        assertEquals(48, coverage.values.count { it.kind.wireName == "menu" })
        assertEquals("chatgpt-page-actions:48", coverage.getValue("feature_48").nativeTriggerSelector)
    }

    @Test
    fun coversTheFullProtocolControlBoundaryWithoutDroppingPageActions() {
        val controls = (1..512).map { index ->
            control(
                id = "feature_$index",
                semantic = "action",
                label = "功能 $index",
                region = ChatGptWebUiRegion.CONTENT,
            )
        }

        val actions = ChatGptNativeControlPresentation.pageActions(controls)
        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(512, actions.size)
        assertEquals(512, coverage.values.count { it.kind.wireName == "menu" })
        assertEquals("chatgpt-page-actions:512", coverage.getValue("feature_512").nativeTriggerSelector)
    }

    @Test
    fun movesSuggestionOverflowIntoTheNativeMenu() {
        val controls = (1..5).map { index ->
            control(
                id = "suggestion_$index",
                semantic = "suggestion",
                label = "建议 $index",
                region = ChatGptWebUiRegion.SUGGESTIONS,
            )
        }

        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(4, ChatGptNativeControlPresentation.suggestions(controls).size)
        assertEquals("menu", coverage.getValue("suggestion_5").kind.wireName)
        assertEquals(
            "chatgpt-overflow-actions:1",
            coverage.getValue("suggestion_5").nativeTriggerSelector,
        )
    }

    @Test
    fun exposesProjectSuggestionsAsStableNativeControls() {
        val project = control(
            id = "project_home",
            semantic = "project",
            label = "我的项目",
            region = ChatGptWebUiRegion.SUGGESTIONS,
        )

        val coverage = ChatGptNativeControlPresentation.describe(listOf(project)).getValue(project.id)

        assertEquals(listOf(project), ChatGptNativeControlPresentation.suggestions(listOf(project)))
        assertEquals("direct", coverage.kind.wireName)
        assertEquals("chatgpt-project:project_home", coverage.nativeSelector)
        assertEquals(
            coverage.nativeSelector,
            ChatGptNativeControlPresentation.directSelector(project),
        )
    }

    @Test
    fun exposesMediaAndReasoningAsMessageActionsWhileVoiceModeUsesOfficialFallback() {
        val controls = listOf(
            control("media", "open_media", "打开图片", ChatGptWebUiRegion.MESSAGE, CONTEXT),
            control("reasoning", "reasoning_details", "思考了 1m 48s", ChatGptWebUiRegion.MESSAGE, CONTEXT),
            control("voice", "voice_mode", "启动语音功能", ChatGptWebUiRegion.COMPOSER),
        )

        val actions = ChatGptNativeControlPresentation.messageActions(controls).getValue(CONTEXT)
        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(listOf("media", "reasoning"), actions.map(ChatGptWebUiControl::id))
        assertEquals("menu", coverage.getValue("media").kind.wireName)
        assertEquals("menu", coverage.getValue("reasoning").kind.wireName)
        assertEquals("official_fallback", coverage.getValue("voice").kind.wireName)
        assertNull(coverage.getValue("voice").nativeSelector)
    }

    @Test
    fun mapsDedicatedControlsToTheirRealNativeSurfaceAndRequiredTrigger() {
        val controls = listOf(
            control("navigation", "navigation", "会话", ChatGptWebUiRegion.HEADER),
            control("new", "new_conversation", "新聊天", ChatGptWebUiRegion.HEADER),
            control("attachment", "attachment", "添加", ChatGptWebUiRegion.COMPOSER),
            control("model", "model", "模型", ChatGptWebUiRegion.COMPOSER),
            control("conversation", "conversation", "工作", ChatGptWebUiRegion.OVERLAY),
            control("project", "project", "项目", ChatGptWebUiRegion.OVERLAY),
        )

        val coverage = ChatGptNativeControlPresentation.describe(controls)

        assertEquals(
            ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
            coverage.getValue("navigation").nativeSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.NEW_CONVERSATION,
            coverage.getValue("new").nativeSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
            coverage.getValue("new").nativeTriggerSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.COMPOSER_TOOLS_TRIGGER,
            coverage.getValue("attachment").nativeSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.COMPOSER_MODEL_TRIGGER,
            coverage.getValue("model").nativeSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
            coverage.getValue("conversation").nativeTriggerSelector,
        )
        assertEquals(
            ChatGptNativeNavigationSelector.FEATURE_LIST_TRIGGER,
            coverage.getValue("project").nativeTriggerSelector,
        )
    }

    private fun control(
        id: String,
        semantic: String,
        label: String,
        region: String,
        contextId: String? = null,
        enabled: Boolean = true,
    ) = ChatGptWebUiControl(
        id = id,
        semantic = semantic,
        label = label,
        region = region,
        role = "button",
        enabled = enabled,
        selected = false,
        contextId = contextId,
    )

    private companion object {
        const val CONTEXT = "conversation-turn-2"
    }
}

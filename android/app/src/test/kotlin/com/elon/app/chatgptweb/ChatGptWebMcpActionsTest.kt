package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpActionsTest {
    @Test
    fun stateExportsConversationContextAndStableControlMetadata() {
        val actions = actions()

        val state = actions.uiState()
        val conversation = state.getJSONObject("conversation")
        val control = state.getJSONObject("ui_manifest").getJSONArray("controls").getJSONObject(0)

        assertEquals("chatgpt_web", state.getString("surface"))
        assertEquals(ChatGptWebPageAdapter.ADAPTER_VERSION, state.getInt("adapter_version"))
        assertEquals("完整回答内容", conversation.getJSONArray("messages").getJSONObject(0).getString("content"))
        assertEquals("control_suggestion_demo", control.getString("control_id"))
        assertEquals(0.25, control.getDouble("web_x_ratio"), 0.0)
        assertFalse(control.getBoolean("in_viewport"))
        assertEquals(1, state.getJSONObject("navigation").getInt("conversation_count"))
        assertEquals(
            "chatgpt-control:control_suggestion_demo:整理待办",
            control.getString("adb_content_description"),
        )
        assertEquals("direct", control.getString("native_presentation"))
        assertEquals(
            "chatgpt-control:control_suggestion_demo:整理待办",
            control.getString("native_adb_content_description"),
        )
    }

    @Test
    fun controlInvokesOnlyIdsPresentInTheCurrentManifest() {
        var invoked = ""
        val actions = actions(onInvoke = { invoked = it })

        val ok = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_suggestion_demo"))
        val stale = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_suggestion_stale"))

        assertTrue(ok.getBoolean("control_ok"))
        assertEquals("control_suggestion_demo", invoked)
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_control_id", stale.getString("error"))
    }

    @Test
    fun contextCanBeReadInStablePages() {
        val result = actions().control(JSONObject()
            .put("action", "chatgpt_get_context")
            .put("message_offset", 0)
            .put("message_limit", 1))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals(1, result.getInt("message_count"))
        assertEquals(0, result.getJSONArray("messages").getJSONObject(0).getInt("index"))
        assertEquals("完整回答内容", result.getJSONArray("messages").getJSONObject(0).getString("content"))
        assertFalse(result.getBoolean("has_more"))
    }

    @Test
    fun controlsAndConversationsCanBeFoundWithoutParsingTheWholeUiState() {
        val actions = actions()

        val controls = actions.control(JSONObject()
            .put("action", "chatgpt_find_controls")
            .put("semantic", "suggestion")
            .put("query", "待办"))
        val conversations = actions.control(JSONObject()
            .put("action", "chatgpt_get_conversations")
            .put("query", "验证"))

        assertEquals(1, controls.getInt("match_count"))
        assertEquals("control_suggestion_demo", controls.getJSONArray("controls")
            .getJSONObject(0).getString("control_id"))
        assertEquals(1, conversations.getInt("match_count"))
        val conversation = conversations.getJSONArray("conversations").getJSONObject(0)
        assertEquals("/c/demo", conversation.getString("path"))
        assertEquals("chatgpt_open_conversation", conversation.getString("native_action"))
        assertEquals(
            "chatgpt-conversation:demo:桥接验证",
            conversation.getString("native_adb_content_description"),
        )

        val messageControl = actions.control(JSONObject()
            .put("action", "chatgpt_find_controls")
            .put("context_id", "conversation-turn-1"))
        assertEquals(1, messageControl.getInt("match_count"))
        assertEquals(
            "menu",
            messageControl.getJSONArray("controls").getJSONObject(0).getString("native_presentation"),
        )
    }

    @Test
    fun navigationEntriesShareStableAdbSelectorsWithDirectMcpActions() {
        var requestedSection = ""
        var selectedOption = ""
        var requestedFeatures = false
        var selectedFeature = ""
        val actions = actions(
            onRequestComposerOptions = { requestedSection = it },
            onSelectComposerOption = { section, id -> selectedOption = "$section:$id" },
            onRequestFeatures = { requestedFeatures = true },
            onSelectFeature = { selectedFeature = it },
        )

        val navigation = actions.control(JSONObject()
            .put("action", "chatgpt_get_navigation")
            .put("section", "model"))
        val feature = navigation.getJSONArray("features").getJSONObject(0)
        val option = navigation.getJSONObject("composer_sections")
            .getJSONArray("model").getJSONObject(0)

        assertEquals("elon.chatgpt_web.navigation.v2", navigation.getString("schema"))
        assertEquals(
            ChatGptNativeNavigationSelector.SCHEMA,
            navigation.getString("native_selector_schema"),
        )
        assertEquals("chatgpt_select_feature", feature.getString("native_action"))
        assertEquals(
            "chatgpt-feature:feature_library:文件库",
            feature.getString("native_adb_content_description"),
        )
        assertEquals("chatgpt_select_composer_option", option.getString("native_action"))
        assertEquals(
            "chatgpt-composer-option:model:model_fast:快速",
            option.getString("native_adb_content_description"),
        )

        assertTrue(actions.control(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", "model")).getBoolean("control_ok"))
        assertTrue(actions.control(JSONObject()
            .put("action", "chatgpt_select_composer_option")
            .put("section", "model")
            .put("option_id", "model_fast")).getBoolean("control_ok"))
        assertTrue(actions.control(JSONObject()
            .put("action", "chatgpt_list_features")).getBoolean("control_ok"))
        assertTrue(actions.control(JSONObject()
            .put("action", "chatgpt_select_feature")
            .put("feature_id", "feature_library")).getBoolean("control_ok"))

        assertEquals("model", requestedSection)
        assertEquals("model:model_fast", selectedOption)
        assertTrue(requestedFeatures)
        assertEquals("feature_library", selectedFeature)
    }

    @Test
    fun directNavigationActionsRejectInvalidSectionsAndStaleIds() {
        val actions = actions()

        val invalidSection = actions.control(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", "attachments"))
        val staleOption = actions.control(JSONObject()
            .put("action", "chatgpt_select_composer_option")
            .put("section", "model")
            .put("option_id", "model_stale"))
        val staleFeature = actions.control(JSONObject()
            .put("action", "chatgpt_select_feature")
            .put("feature_id", "feature_stale"))

        assertEquals("invalid_section", invalidSection.getString("error"))
        assertEquals("stale_option_id", staleOption.getString("error"))
        assertEquals("stale_feature_id", staleFeature.getString("error"))
    }

    @Test
    fun capabilityMatrixIsAvailableThroughTheStableUiControlTool() {
        val matrix = actions().control(JSONObject().put("action", "chatgpt_get_capability_matrix"))

        assertTrue(matrix.getBoolean("control_ok"))
        assertEquals("elon.chatgpt_web.capability_matrix.v2", matrix.getString("schema"))
        assertEquals(ChatGptWebPageAdapter.ADAPTER_VERSION, matrix.getInt("adapter_version"))
        assertTrue(matrix.getBoolean("ready_for_chat"))
    }

    private fun actions(
        onInvoke: (String) -> Unit = {},
        onRequestComposerOptions: (String) -> Unit = {},
        onSelectComposerOption: (String, String) -> Unit = { _, _ -> },
        onRequestFeatures: () -> Unit = {},
        onSelectFeature: (String) -> Unit = {},
    ): ChatGptWebMcpActions {
        val snapshot = ChatGptWebSnapshot(
            title = "工作",
            url = "https://chatgpt.com/c/demo",
            draft = "",
            messages = listOf(ChatGptWebMessage("a1", "assistant", "完整回答内容", "completed", emptyList())),
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "5.6 Sol 轻度",
            attachments = emptyList(),
            dictationActive = false,
            capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.DRAFT_SYNC)),
        )
        val manifest = ChatGptWebUiManifest(
            version = 1,
            pageKind = "conversation",
            title = "工作",
            compatibility = "healthy",
            controls = listOf(
                ChatGptWebUiControl(
                    id = "control_suggestion_demo",
                    semantic = "suggestion",
                    label = "整理待办",
                    region = ChatGptWebUiRegion.SUGGESTIONS,
                    role = "button",
                    enabled = true,
                    selected = false,
                    inViewport = false,
                    webXRatio = 0.25,
                    webYRatio = 0.75,
                ),
                ChatGptWebUiControl(
                    id = "control_share_demo",
                    semantic = "share",
                    label = "分享",
                    region = ChatGptWebUiRegion.MESSAGE,
                    role = "button",
                    enabled = true,
                    selected = false,
                    contextId = "conversation-turn-1",
                ),
            ),
        )
        return ChatGptWebMcpActions(
            snapshot = { snapshot },
            uiManifest = { manifest },
            observedState = {
                ChatGptWebObservedState.Snapshot(
                    conversations = listOf(
                        ChatGptWebConversation("demo", "桥接验证", "/c/demo", active = true),
                    ),
                    features = listOf(
                        ChatGptWebFeature("feature_library", "文件库", "library", selected = false),
                    ),
                    composerSections = mapOf(
                        "model" to listOf(
                            ChatGptWebComposerOption(
                                "model_fast",
                                "快速",
                                selected = false,
                                kind = "menuitemradio",
                            ),
                        ),
                    ),
                    lastCommand = ChatGptWebEvent.CommandResult("list_conversations", true, ""),
                    updatedAtMs = 123L,
                )
            },
            bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebModeController.Mode.NATIVE },
            inputText = { "" },
            setInputText = {},
            sendInput = {},
            invokeControl = onInvoke,
            newConversation = {},
            stopGeneration = {},
            refresh = {},
            refreshControls = {},
            selectMode = {},
            openConversation = {},
            listConversations = {},
            requestComposerOptions = onRequestComposerOptions,
            selectComposerOption = onSelectComposerOption,
            requestFeatures = onRequestFeatures,
            selectFeature = onSelectFeature,
        )
    }
}

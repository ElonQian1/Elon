package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpActionsTest {
    @Test
    fun openingChatGptWebIsIdempotentWhenTheSurfaceIsAlreadyActive() {
        val result = actions().control(JSONObject().put("action", "open_chatgpt_web"))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("open_chatgpt_web", result.getString("action"))
        assertEquals("chatgpt_web", result.getString("surface"))
    }

    @Test
    fun stateExportsConversationContextAndStableControlMetadata() {
        val actions = actions()

        val state = actions.uiState()
        val conversation = state.getJSONObject("conversation")
        val control = state.getJSONObject("ui_manifest").getJSONArray("controls").getJSONObject(0)

        assertEquals("chatgpt_web", state.getString("surface"))
        assertEquals(ChatGptWebPageAdapter.ADAPTER_VERSION, state.getInt("adapter_version"))
        assertEquals("conversation", state.getString("page_kind"))
        assertFalse(state.getBoolean("login_required"))
        assertEquals("完整回答内容", conversation.getJSONArray("messages").getJSONObject(0).getString("content"))
        val messageParts = conversation.getJSONArray("messages").getJSONObject(0).getJSONArray("parts")
        assertEquals(2, messageParts.length())
        assertEquals("image", messageParts.getJSONObject(0).getString("type"))
        assertEquals("生成的图片", messageParts.getJSONObject(0).getString("label"))
        assertEquals("control_suggestion_demo", control.getString("control_id"))
        assertEquals(0.25, control.getDouble("web_x_ratio"), 0.0)
        assertFalse(control.getBoolean("in_viewport"))
        assertEquals(1, state.getJSONObject("navigation").getInt("conversation_count"))
        assertEquals(123L, state.getJSONObject("last_command").getLong("observed_at_ms"))
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
        assertEquals("dispatched", ok.getString("command_status"))
        assertEquals(
            "invoke_ui_control",
            ok.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertEquals("pending", ok.getJSONObject("command_receipt").getString("status"))
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
        val message = result.getJSONArray("messages").getJSONObject(0)
        assertEquals(2, message.getInt("part_count"))
        assertFalse(message.getBoolean("parts_truncated"))
        assertEquals("file", message.getJSONArray("parts").getJSONObject(1).getString("type"))
        assertFalse(result.getBoolean("has_more"))
    }

    @Test
    fun structuredPartsStayBoundedWhileReportingTheOriginalCounts() {
        val parts = (1..20).map { index ->
            ChatGptWebMessagePart("file", "x".repeat(200) + index)
        }
        val message = actions(messageParts = parts).control(JSONObject()
            .put("action", "chatgpt_get_context"))
            .getJSONArray("messages")
            .getJSONObject(0)

        assertEquals(20, message.getInt("part_count"))
        assertTrue(message.getBoolean("parts_truncated"))
        assertEquals(16, message.getJSONArray("parts").length())
        assertEquals(180, message.getJSONArray("parts").getJSONObject(0).getString("label").length)
        assertTrue(message.getJSONArray("parts").getJSONObject(0).getBoolean("label_truncated"))
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

    @Test
    fun activeDictationExportsStateAndAllowsOnlyScopedSessionActions() {
        var cancellations = 0
        var submissions = 0
        val active = actions(
            dictationActive = true,
            onCancelDictation = { cancellations += 1 },
            onSubmitDictation = { submissions += 1 },
        )

        assertTrue(active.uiState().getBoolean("dictation_active"))
        assertTrue(active.control(JSONObject().put("action", "chatgpt_cancel_dictation"))
            .getBoolean("control_ok"))
        assertTrue(active.control(JSONObject().put("action", "chatgpt_submit_dictation"))
            .getBoolean("control_ok"))
        assertEquals(1, cancellations)
        assertEquals(1, submissions)

        val inactive = actions().control(JSONObject().put("action", "chatgpt_cancel_dictation"))
        assertFalse(inactive.getBoolean("control_ok"))
        assertEquals("dictation_not_active", inactive.getString("error"))
    }

    private fun actions(
        dictationActive: Boolean = false,
        messageParts: List<ChatGptWebMessagePart>? = null,
        onInvoke: (String) -> Unit = {},
        onCancelDictation: () -> Unit = {},
        onSubmitDictation: () -> Unit = {},
        onRequestComposerOptions: (String) -> Unit = {},
        onSelectComposerOption: (String, String) -> Unit = { _, _ -> },
        onRequestFeatures: () -> Unit = {},
        onSelectFeature: (String) -> Unit = {},
    ): ChatGptWebMcpActions {
        val snapshot = ChatGptWebSnapshot(
            title = "工作",
            url = "https://chatgpt.com/c/demo",
            draft = "",
            messages = listOf(ChatGptWebMessage(
                id = "a1",
                role = "assistant",
                content = "完整回答内容",
                state = "completed",
                parts = messageParts ?: listOf(
                    ChatGptWebMessagePart("image", "生成的图片"),
                    ChatGptWebMessagePart("file", "分析结果.csv"),
                ),
            )),
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "5.6 Sol 轻度",
            attachments = emptyList(),
            dictationActive = dictationActive,
            capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.DRAFT_SYNC)),
            pageKind = "conversation",
            loginRequired = false,
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
        var nextCommandId = 0
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
                    commandRequests = emptyList(),
                    updatedAtMs = 123L,
                    lastCommandObservedAtMs = 123L,
                )
            },
            beginCommand = { expectedAction ->
                ChatGptWebObservedState.CommandRequest(
                    id = "mcp_${++nextCommandId}",
                    expectedAction = expectedAction,
                    status = ChatGptWebObservedState.CommandRequest.PENDING,
                    startedAtMs = 123L,
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
            cancelDictation = onCancelDictation,
            submitDictation = onSubmitDictation,
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

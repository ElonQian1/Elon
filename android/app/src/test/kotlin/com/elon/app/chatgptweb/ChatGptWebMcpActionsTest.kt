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
        assertTrue(state.getJSONObject("navigation")
            .getJSONObject("conversation_collection")
            .getBoolean("reached_end"))
        assertEquals(2, state.getJSONObject("ui_manifest").getInt("discovered_control_count"))
        assertFalse(state.getJSONObject("ui_manifest").getBoolean("controls_truncated"))
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
        var dispatchedRequestId = ""
        val actions = actions(
            onInvoke = { invoked = it },
            onDispatch = { action, requestId ->
                if (action == "invoke_ui_control") dispatchedRequestId = requestId
            },
        )

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
        assertEquals(
            ok.getJSONObject("command_receipt").getString("request_id"),
            dispatchedRequestId,
        )
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_control_id", stale.getString("error"))
    }

    @Test
    fun writablePageTextControlsUseAReceiptWithoutExposingExistingValues() {
        var target = ""
        var written = ""
        var dispatched = ""
        val actions = actions(
            includeWritableControl = true,
            onSetControlText = { controlId, text ->
                target = controlId
                written = text
            },
            onDispatch = { action, _ -> dispatched = action },
        )

        val result = actions.control(
            JSONObject()
                .put("action", "chatgpt_set_control_text")
                .put("control_id", "control_search_demo")
                .put("text", "release notes"),
        )
        val control = actions.uiState().getJSONObject("ui_manifest")
            .getJSONArray("controls").getJSONObject(2)

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("control_search_demo", target)
        assertEquals("release notes", written)
        assertEquals("set_ui_control_text", dispatched)
        assertEquals("search", control.getString("input_kind"))
        assertTrue(control.getBoolean("writable"))
        assertFalse(control.has("value"))
        assertEquals(
            "chatgpt-control-input:control_search_demo",
            control.getString("native_value_input_content_description"),
        )
        assertEquals(
            "chatgpt-control-input-commit:control_search_demo",
            control.getString("native_value_commit_content_description"),
        )

        val rejected = actions.control(
            JSONObject()
                .put("action", "chatgpt_set_control_text")
                .put("control_id", "control_suggestion_demo")
                .put("text", "blocked"),
        )
        assertFalse(rejected.getBoolean("control_ok"))
        assertEquals("control_not_writable", rejected.getString("error"))
    }

    @Test
    fun stateAndChoiceControlsDispatchIdempotentCommandsWithoutPrivateValues() {
        var selectedTarget: Pair<String, Boolean>? = null
        var choiceTarget: Pair<String, Int>? = null
        val actions = actions(
            includeFormControls = true,
            onSetControlSelected = { id, selected -> selectedTarget = id to selected },
            onSelectControlChoice = { id, index -> choiceTarget = id to index },
        )

        val selected = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_selected")
            .put("control_id", "control_toggle_demo")
            .put("selected", true))
        val choice = actions.control(JSONObject()
            .put("action", "chatgpt_select_control_choice")
            .put("control_id", "control_model_demo")
            .put("choice_index", 1))
        val controls = actions.uiState().getJSONObject("ui_manifest").getJSONArray("controls")
        val toggle = controls.getJSONObject(2)
        val model = controls.getJSONObject(3)

        assertTrue(selected.getBoolean("control_ok"))
        assertEquals("control_toggle_demo" to true, selectedTarget)
        assertTrue(choice.getBoolean("control_ok"))
        assertEquals("control_model_demo" to 1, choiceTarget)
        assertTrue(toggle.getBoolean("state_settable"))
        assertEquals(2, model.getJSONArray("choice_labels").length())
        assertEquals(0, model.getInt("selected_choice_index"))
        assertFalse(model.has("value"))
        assertEquals(
            "chatgpt-control-choice:control_model_demo:1",
            model.getJSONArray("native_choice_content_descriptions").getString(1),
        )

        val missingState = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_selected")
            .put("control_id", "control_toggle_demo"))
        val fractionalChoice = actions.control(JSONObject()
            .put("action", "chatgpt_select_control_choice")
            .put("control_id", "control_model_demo")
            .put("choice_index", 1.5))
        assertFalse(missingState.getBoolean("control_ok"))
        assertEquals("missing_selected", missingState.getString("error"))
        assertFalse(fractionalChoice.getBoolean("control_ok"))
        assertEquals("invalid_choice_index", fractionalChoice.getString("error"))
    }

    @Test
    fun nativeSlidersExposeBoundsAndDispatchTargetValues() {
        var sliderTarget: Pair<String, Double>? = null
        val actions = actions(
            includeSliderControl = true,
            onSetControlSlider = { id, value -> sliderTarget = id to value },
        )

        val result = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_slider")
            .put("control_id", "control_effort_demo")
            .put("value", 1.5))
        val slider = actions.uiState().getJSONObject("ui_manifest")
            .getJSONArray("controls").getJSONObject(2).getJSONObject("slider")

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("control_effort_demo" to 1.5, sliderTarget)
        assertEquals(0.0, slider.getDouble("min"), 0.0)
        assertEquals(2.0, slider.getDouble("max"), 0.0)
        assertEquals(0.5, slider.getDouble("step"), 0.0)
        assertEquals(1.0, slider.getDouble("value"), 0.0)
        assertEquals(
            "chatgpt-control-slider:control_effort_demo",
            slider.getString("native_input_content_description"),
        )

        val rejected = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_slider")
            .put("control_id", "control_effort_demo")
            .put("value", 3))
        assertFalse(rejected.getBoolean("control_ok"))
        assertEquals("slider_value_out_of_range", rejected.getString("error"))
    }

    @Test
    fun unsupportedAriaSlidersRejectBlindGenericInvocation() {
        val actions = actions(includeUnsupportedSlider = true)

        val result = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_aria_slider_demo"))

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("control_requires_official_fallback", result.getString("error"))
    }

    @Test
    fun disclosureControlsDispatchDesiredExpandedState() {
        var expandedTarget: Pair<String, Boolean>? = null
        val actions = actions(
            includeExpandedControl = true,
            onSetControlExpanded = { id, expanded -> expandedTarget = id to expanded },
        )

        val result = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_expanded")
            .put("control_id", "control_projects_demo")
            .put("expanded", true))
        val control = actions.uiState().getJSONObject("ui_manifest")
            .getJSONArray("controls").getJSONObject(2)

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("control_projects_demo" to true, expandedTarget)
        assertFalse(control.getBoolean("expanded"))
        assertTrue(control.getBoolean("expandable"))

        val missing = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_expanded")
            .put("control_id", "control_projects_demo"))
        assertFalse(missing.getBoolean("control_ok"))
        assertEquals("missing_expanded", missing.getString("error"))
    }

    @Test
    fun everyAcknowledgedWebCommandPassesItsReceiptIdToTheCommandPort() {
        val dispatched = mutableListOf<Pair<String, String>>()
        val actions = actions(
            dictationActive = true,
            regenerateSupported = true,
            includeWritableControl = true,
            includeFormControls = true,
            includeSliderControl = true,
            includeExpandedControl = true,
            includeRealtimeVoiceControl = true,
            onDispatch = { action, requestId -> dispatched += action to requestId },
        )
        val commands = listOf(
            JSONObject().put("action", "send_input") to "send_prompt",
            JSONObject().put("action", "chatgpt_invoke_control")
                .put("control_id", "control_suggestion_demo") to "invoke_ui_control",
            JSONObject().put("action", "chatgpt_set_control_text")
                .put("control_id", "control_search_demo")
                .put("text", "release notes") to "set_ui_control_text",
            JSONObject().put("action", "chatgpt_set_control_selected")
                .put("control_id", "control_toggle_demo")
                .put("selected", true) to "set_ui_control_selected",
            JSONObject().put("action", "chatgpt_select_control_choice")
                .put("control_id", "control_model_demo")
                .put("choice_index", 1) to "select_ui_control_choice",
            JSONObject().put("action", "chatgpt_set_control_slider")
                .put("control_id", "control_effort_demo")
                .put("value", 1.5) to "set_ui_control_slider",
            JSONObject().put("action", "chatgpt_set_control_expanded")
                .put("control_id", "control_projects_demo")
                .put("expanded", true) to "set_ui_control_expanded",
            JSONObject().put("action", "chatgpt_new_conversation") to "new_conversation",
            JSONObject().put("action", "chatgpt_stop_generation") to "stop_generation",
            JSONObject().put("action", "chatgpt_regenerate_response") to "regenerate_response",
            JSONObject().put("action", "chatgpt_start_realtime_voice") to "invoke_ui_control",
            JSONObject().put("action", "chatgpt_cancel_dictation") to "cancel_dictation",
            JSONObject().put("action", "chatgpt_submit_dictation") to "submit_dictation",
            JSONObject().put("action", "chatgpt_refresh_controls") to "snapshot_ui_manifest",
            JSONObject().put("action", "chatgpt_list_conversations") to "list_conversations",
            JSONObject().put("action", "chatgpt_list_composer_options")
                .put("section", "model") to "list_model_options",
            JSONObject().put("action", "chatgpt_select_composer_option")
                .put("section", "model")
                .put("option_id", "model_fast") to "select_model_option",
            JSONObject().put("action", "chatgpt_list_features") to "list_navigation",
            JSONObject().put("action", "chatgpt_select_feature")
                .put("feature_id", "feature_library") to "select_navigation",
            JSONObject().put("action", "chatgpt_open_conversation")
                .put("conversation_path", "/c/demo") to "open_conversation",
        )

        commands.forEach { (args, expectedAction) ->
            val response = actions.control(args)
            val receipt = response.getJSONObject("command_receipt")
            assertTrue(response.getBoolean("control_ok"))
            assertEquals(expectedAction, receipt.getString("expected_web_action"))
            assertEquals(expectedAction, dispatched.last().first)
            assertEquals(receipt.getString("request_id"), dispatched.last().second)
        }
    }

    @Test
    fun contextCanBeReadInStablePages() {
        val result = actions().control(JSONObject()
            .put("action", "chatgpt_get_context")
            .put("message_offset", 0)
            .put("message_limit", 1))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("elon.chatgpt_web.context.v2", result.getString("schema"))
        assertEquals(24, result.getString("context_revision").length)
        assertTrue(result.getString("message_cursor").startsWith("ctx1."))
        assertTrue(result.isNull("next_message_cursor"))
        assertTrue(result.getBoolean("cursor_stable"))
        assertEquals(1, result.getInt("message_count"))
        assertEquals(0, result.getJSONArray("messages").getJSONObject(0).getInt("index"))
        assertEquals("完整回答内容", result.getJSONArray("messages").getJSONObject(0).getString("content"))
        val message = result.getJSONArray("messages").getJSONObject(0)
        assertEquals(2, message.getInt("part_count"))
        assertFalse(message.getBoolean("parts_truncated"))
        assertEquals("file", message.getJSONArray("parts").getJSONObject(1).getString("type"))
        assertEquals("chatgpt_reveal_message", message.getString("native_action"))
        assertEquals(
            listOf("message", "content", "copy", "regenerate", "actions"),
            message.getJSONArray("native_reveal_targets").let { targets ->
                (0 until targets.length()).map(targets::getString)
            },
        )
        assertEquals("chatgpt-message:a0:assistant", message.getString("native_adb_content_description"))
        assertEquals(
            "chatgpt-message-part:a0:1:file",
            message.getJSONArray("parts").getJSONObject(1)
                .getString("native_adb_content_description"),
        )
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
    fun contextReportsTheObservedWindowWithoutRenumberingMessages() {
        val result = actions(
            messageWindowStart = 80,
            observedMessageCount = 81,
        ).control(JSONObject().put("action", "chatgpt_get_context"))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals(81, result.getInt("message_count"))
        assertEquals(1, result.getInt("available_message_count"))
        assertEquals(80, result.getInt("message_window_start"))
        assertEquals(81, result.getInt("message_window_end"))
        assertTrue(result.getBoolean("history_truncated"))
        assertTrue(result.getBoolean("has_more_before"))
        assertFalse(result.getBoolean("context_complete"))
        assertEquals(80, result.getJSONArray("messages").getJSONObject(0).getInt("index"))
    }

    @Test
    fun uiStateConversationReportsGlobalWindowBoundsAndExportOffsets() {
        val conversation = actions(
            messageWindowStart = 20,
            availableMessageCount = 80,
            observedMessageCount = 100,
        ).uiState().getJSONObject("conversation")
        val messages = conversation.getJSONArray("messages")

        assertEquals("elon.chatgpt_web.conversation_summary.v2", conversation.getString("schema"))
        assertEquals(100, conversation.getInt("message_count"))
        assertEquals(80, conversation.getInt("available_message_count"))
        assertEquals(20, conversation.getInt("message_window_start"))
        assertEquals(100, conversation.getInt("message_window_end"))
        assertTrue(conversation.getBoolean("history_truncated"))
        assertFalse(conversation.getBoolean("context_complete"))
        assertEquals(50, conversation.getInt("exported_message_count"))
        assertEquals(50, conversation.getInt("exported_message_offset"))
        assertTrue(conversation.getBoolean("messages_truncated"))
        assertEquals("chatgpt_get_context", conversation.getString("context_action"))
        assertEquals(50, messages.getJSONObject(0).getInt("index"))
        assertEquals(99, messages.getJSONObject(messages.length() - 1).getInt("index"))
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

        assertEquals(2, controls.getInt("manifest_control_count"))
        assertEquals(2, controls.getInt("discovered_control_count"))
        assertFalse(controls.getBoolean("controls_truncated"))

        assertEquals(1, controls.getInt("match_count"))
        assertEquals("control_suggestion_demo", controls.getJSONArray("controls")
            .getJSONObject(0).getString("control_id"))
        assertEquals(1, conversations.getInt("match_count"))
        assertEquals(1, conversations.getInt("source_count"))
        assertTrue(conversations.getJSONObject("collection").getBoolean("scroll_restored"))
        assertTrue(conversations.getJSONObject("collection").getBoolean("reached_end"))
        assertTrue(conversations.getJSONObject("collection").getBoolean("complete"))
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
        assertEquals("model", option.getString("semantic"))
        assertTrue(option.getBoolean("opens_submenu"))
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

    @Test
    fun directDictationStartRequiresCapabilityAndReturnsAReceipt() {
        var starts = 0
        var dispatchedRequestId = ""
        val supported = actions(
            dictationSupported = true,
            onStartDictation = { starts += 1 },
            onDispatch = { action, requestId ->
                if (action == "start_dictation") dispatchedRequestId = requestId
            },
        )

        val started = supported.control(JSONObject().put("action", "chatgpt_start_dictation"))

        assertTrue(started.getBoolean("control_ok"))
        assertEquals(1, starts)
        assertEquals(
            "start_dictation",
            started.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertEquals(
            started.getJSONObject("command_receipt").getString("request_id"),
            dispatchedRequestId,
        )

        val unsupported = actions().control(JSONObject().put("action", "chatgpt_start_dictation"))
        assertFalse(unsupported.getBoolean("control_ok"))
        assertEquals("dictation_unavailable", unsupported.getString("error"))

        val active = actions(
            dictationActive = true,
            dictationSupported = true,
        ).control(JSONObject().put("action", "chatgpt_start_dictation"))
        assertFalse(active.getBoolean("control_ok"))
        assertEquals("dictation_already_active", active.getString("error"))
    }

    @Test
    fun realtimeVoiceRequiresAnEnabledSemanticControlAndReturnsAReceipt() {
        var invokedControl = ""
        var dispatchedRequestId = ""
        val supported = actions(
            includeRealtimeVoiceControl = true,
            onInvoke = { invokedControl = it },
            onDispatch = { action, requestId ->
                if (action == "invoke_ui_control") dispatchedRequestId = requestId
            },
        )

        val started = supported.control(JSONObject().put("action", "chatgpt_start_realtime_voice"))

        assertTrue(started.getBoolean("control_ok"))
        assertEquals("control_realtime_voice", invokedControl)
        assertEquals(
            "invoke_ui_control",
            started.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertEquals(
            started.getJSONObject("command_receipt").getString("request_id"),
            dispatchedRequestId,
        )

        val unavailable = actions().control(JSONObject().put("action", "chatgpt_start_realtime_voice"))
        assertFalse(unavailable.getBoolean("control_ok"))
        assertEquals("realtime_voice_unavailable", unavailable.getString("error"))
    }

    @Test
    fun directAttachmentRemovalRequiresFreshRemovableAttachmentAndReturnsAReceipt() {
        val removable = ChatGptWebAttachment("attachment_demo", "smoke.png", "ready", true)
        val uploading = ChatGptWebAttachment("attachment_uploading", "pending.txt", "uploading", false)
        var removedId = ""
        var dispatchedRequestId = ""
        val actions = actions(
            attachments = listOf(removable, uploading),
            onRemoveAttachment = { removedId = it },
            onDispatch = { action, requestId ->
                if (action == "remove_attachment") dispatchedRequestId = requestId
            },
        )

        val removed = actions.control(JSONObject()
            .put("action", "chatgpt_remove_attachment")
            .put("attachment_id", removable.id))

        assertTrue(removed.getBoolean("control_ok"))
        assertEquals(removable.id, removedId)
        assertEquals(
            "remove_attachment",
            removed.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertEquals(
            removed.getJSONObject("command_receipt").getString("request_id"),
            dispatchedRequestId,
        )

        val stale = actions.control(JSONObject()
            .put("action", "chatgpt_remove_attachment")
            .put("attachment_id", "attachment_stale"))
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_attachment_id", stale.getString("error"))

        val pending = actions.control(JSONObject()
            .put("action", "chatgpt_remove_attachment")
            .put("attachment_id", uploading.id))
        assertFalse(pending.getBoolean("control_ok"))
        assertEquals("attachment_not_removable", pending.getString("error"))

        val invalid = actions.control(JSONObject()
            .put("action", "chatgpt_remove_attachment")
            .put("attachment_id", " "))
        assertFalse(invalid.getBoolean("control_ok"))
        assertEquals("invalid_attachment_id", invalid.getString("error"))
    }

    private fun actions(
        dictationActive: Boolean = false,
        dictationSupported: Boolean = false,
        regenerateSupported: Boolean = false,
        attachments: List<ChatGptWebAttachment> = emptyList(),
        messageParts: List<ChatGptWebMessagePart>? = null,
        messageWindowStart: Int = 0,
        availableMessageCount: Int = 1,
        observedMessageCount: Int = messageWindowStart + availableMessageCount,
        onInvoke: (String) -> Unit = {},
        includeWritableControl: Boolean = false,
        includeFormControls: Boolean = false,
        includeSliderControl: Boolean = false,
        includeUnsupportedSlider: Boolean = false,
        includeExpandedControl: Boolean = false,
        includeRealtimeVoiceControl: Boolean = false,
        onSetControlText: (String, String) -> Unit = { _, _ -> },
        onSetControlSelected: (String, Boolean) -> Unit = { _, _ -> },
        onSelectControlChoice: (String, Int) -> Unit = { _, _ -> },
        onSetControlSlider: (String, Double) -> Unit = { _, _ -> },
        onSetControlExpanded: (String, Boolean) -> Unit = { _, _ -> },
        onStartDictation: () -> Unit = {},
        onCancelDictation: () -> Unit = {},
        onSubmitDictation: () -> Unit = {},
        onRemoveAttachment: (String) -> Unit = {},
        onRequestComposerOptions: (String) -> Unit = {},
        onSelectComposerOption: (String, String) -> Unit = { _, _ -> },
        onRequestFeatures: () -> Unit = {},
        onSelectFeature: (String) -> Unit = {},
        onDispatch: (String, String) -> Unit = { _, _ -> },
    ): ChatGptWebMcpActions {
        val snapshot = ChatGptWebSnapshot(
            title = "工作",
            url = "https://chatgpt.com/c/demo",
            draft = "",
            messages = List(availableMessageCount) { index ->
                ChatGptWebMessage(
                    id = "a$index",
                    role = "assistant",
                    content = if (index == 0) "完整回答内容" else "回答 $index",
                    state = "completed",
                    parts = if (index == 0) {
                        messageParts ?: listOf(
                            ChatGptWebMessagePart("image", "生成的图片"),
                            ChatGptWebMessagePart("file", "分析结果.csv"),
                        )
                    } else {
                        emptyList()
                    },
                )
            },
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "5.6 Sol 轻度",
            attachments = attachments,
            dictationActive = dictationActive,
            capabilities = ChatGptWebCapabilities(buildSet {
                add(ChatGptWebCapabilityId.DRAFT_SYNC)
                if (dictationSupported) add(ChatGptWebCapabilityId.DICTATION)
                if (regenerateSupported) add(ChatGptWebCapabilityId.MESSAGE_REGENERATE)
            }),
            pageKind = "conversation",
            loginRequired = false,
            messageWindowStart = messageWindowStart,
            observedMessageCount = observedMessageCount,
        )
        val manifest = ChatGptWebUiManifest(
            version = 1,
            pageKind = "conversation",
            title = "工作",
            compatibility = "healthy",
            controls = buildList {
                add(ChatGptWebUiControl(
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
                ))
                add(ChatGptWebUiControl(
                    id = "control_share_demo",
                    semantic = "share",
                    label = "分享",
                    region = ChatGptWebUiRegion.MESSAGE,
                    role = "button",
                    enabled = true,
                    selected = false,
                    contextId = "conversation-turn-1",
                ))
                if (includeWritableControl) {
                    add(
                        ChatGptWebUiControl(
                            id = "control_search_demo",
                            semantic = "search",
                            label = "搜索聊天",
                            region = ChatGptWebUiRegion.CONTENT,
                            role = "textbox",
                            enabled = true,
                            selected = false,
                            inputKind = "search",
                            writable = true,
                        ),
                    )
                }
                if (includeFormControls) {
                    add(ChatGptWebUiControl(
                        id = "control_toggle_demo",
                        semantic = "toggle",
                        label = "启用记忆",
                        region = ChatGptWebUiRegion.CONTENT,
                        role = "switch",
                        enabled = true,
                        selected = false,
                        inputKind = "switch",
                        stateSettable = true,
                    ))
                    add(ChatGptWebUiControl(
                        id = "control_model_demo",
                        semantic = "selection",
                        label = "模型",
                        region = ChatGptWebUiRegion.CONTENT,
                        role = "combobox",
                        enabled = true,
                        selected = false,
                        inputKind = "select",
                        choiceLabels = listOf("快速", "思考"),
                        selectedChoiceIndex = 0,
                    ))
                }
                if (includeSliderControl) {
                    add(ChatGptWebUiControl(
                        id = "control_effort_demo",
                        semantic = "slider",
                        label = "思考强度",
                        region = ChatGptWebUiRegion.CONTENT,
                        role = "slider",
                        enabled = true,
                        selected = false,
                        inputKind = "range",
                        slider = ChatGptWebSlider(0.0, 2.0, 0.5, 1.0),
                    ))
                }
                if (includeUnsupportedSlider) {
                    add(ChatGptWebUiControl(
                        id = "control_aria_slider_demo",
                        semantic = "slider",
                        label = "自定义强度",
                        region = ChatGptWebUiRegion.CONTENT,
                        role = "slider",
                        enabled = true,
                        selected = false,
                        inputKind = "range",
                    ))
                }
                if (includeExpandedControl) {
                    add(ChatGptWebUiControl(
                        id = "control_projects_demo",
                        semantic = "navigation",
                        label = "项目",
                        region = ChatGptWebUiRegion.CONTENT,
                        role = "treeitem",
                        enabled = true,
                        selected = false,
                        expanded = false,
                        expandable = true,
                    ))
                }
                if (includeRealtimeVoiceControl) {
                    add(ChatGptWebUiControl(
                        id = "control_realtime_voice",
                        semantic = ChatGptRealtimeVoicePolicy.SEMANTIC,
                        label = "实时语音",
                        region = ChatGptWebUiRegion.COMPOSER,
                        role = "button",
                        enabled = true,
                        selected = false,
                    ))
                }
            },
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
                                semantic = "model",
                                opensSubmenu = true,
                            ),
                        ),
                    ),
                    lastCommand = ChatGptWebEvent.CommandResult("list_conversations", true, ""),
                    commandRequests = emptyList(),
                    updatedAtMs = 123L,
                    lastCommandObservedAtMs = 123L,
                    pageGeneration = 1L,
                    adapterGeneration = 1L,
                    conversationCollection = ChatGptWebConversationCollection(
                        scrollerFound = true,
                        scrolled = true,
                        scrollRestored = true,
                        reachedEnd = true,
                        observedCount = 1,
                        steps = 3,
                    ),
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
            commands = ChatGptWebMcpTestCommandPort(
                onInvoke = onInvoke,
                onSetControlText = onSetControlText,
                onSetControlSelected = onSetControlSelected,
                onSelectControlChoice = onSelectControlChoice,
                onSetControlSlider = onSetControlSlider,
                onSetControlExpanded = onSetControlExpanded,
                onStartDictation = onStartDictation,
                onCancelDictation = onCancelDictation,
                onSubmitDictation = onSubmitDictation,
                onRemoveAttachment = onRemoveAttachment,
                onRequestComposerOptions = onRequestComposerOptions,
                onSelectComposerOption = onSelectComposerOption,
                onRequestFeatures = onRequestFeatures,
                onSelectFeature = onSelectFeature,
                onDispatch = onDispatch,
            ),
            refresh = {},
            selectMode = {},
            revealMessage = { _, _, _ -> true },
        )
    }
}

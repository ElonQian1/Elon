package com.elon.app.chatgptweb

import com.elon.app.WebChatSocialMcpPort
import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebMcpActions(
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val uiManifest: () -> ChatGptWebUiManifest?,
    private val observedState: () -> ChatGptWebObservedState.Snapshot,
    private val beginCommand: (String) -> ChatGptWebObservedState.CommandRequest,
    private val bridgeState: () -> ChatGptWebPageAdapter.State,
    private val mode: () -> ChatGptWebPresentationMode,
    private val inputText: () -> String,
    private val audioPermissionState: () -> ChatGptWebAudioPermissionState.Snapshot = { ChatGptWebAudioPermissionState.UNOBSERVED },
    private val verificationEvidence: () -> ChatGptWebVerificationEvidenceStore.Snapshot = { ChatGptWebVerificationEvidenceStore.Snapshot.EMPTY },
    private val recordVerificationCases: (Set<String>) -> ChatGptWebVerificationEvidenceStore.Snapshot = { ChatGptWebVerificationEvidenceStore.Snapshot.EMPTY },
    private val setInputText: (String) -> Unit,
    private val copyMessage: (String) -> ChatGptClipboardMetadata = {
        ChatGptClipboardMetadata(false, 0, emptySet())
    },
    private val commands: ChatGptWebMcpCommandPort,
    private val refresh: () -> Unit,
    private val selectMode: (ChatGptWebPresentationMode) -> Unit,
    private val revealMessage: (String, Int?, String) -> Boolean,
) : WebChatSocialMcpPort {
    override fun uiState(): JSONObject {
        val observed = observedState()
        val current = snapshot().takeIf { observed.adapterCurrent }
        val currentManifest = uiManifest().takeIf { observed.adapterCurrent }
        return JSONObject()
            .put("surface", "chatgpt_web")
            .put("active_page", "chatgpt_web")
            .put("adapter_version", ChatGptWebPageAdapter.ADAPTER_VERSION)
            .put("page_generation", observed.pageGeneration)
            .put("adapter_generation", observed.adapterGeneration)
            .put("adapter_current", observed.adapterCurrent)
            .put("bridge_state", bridgeState().name.lowercase())
            .put("view_mode", mode().name.lowercase())
            .put("authenticated", current?.authenticated ?: false)
            .put("page_kind", current?.pageKind ?: "unknown")
            .put("login_required", current?.loginRequired ?: false)
            .put("composer_ready", current?.composerReady ?: false)
            .put("dictation_active", current?.dictationActive ?: false)
            .put("streaming", current?.streaming ?: false)
            .put("private_stream_observer", JSONObject()
                .put("observed", current?.privateStreamObserved ?: false)
                .put("revision", current?.privateStreamRevision ?: 0L)
                .put("state", current?.privateStreamState ?: "idle")
            )
            .put("conversation", conversationJson(current))
            .put("input", JSONObject()
                .put("text", inputText().take(MAX_INPUT_CHARS))
                .put("text_length", inputText().length)
                .put("official_draft_length", current?.draft?.length ?: 0)
            )
            .put("audio", ChatGptWebAudioPermissionJson.encode(audioPermissionState()))
            .put("ui_manifest", manifestJson(currentManifest))
            .put("navigation", navigationSummary(observed))
            .put("last_command", ChatGptWebCommandReceipts.lastResultJson(observed))
            .put("command_requests", ChatGptWebCommandReceipts.requestsJson(observed))
            .put("available_actions", JSONArray(ChatGptWebMcpActionCatalog.availableActions))
    }

    override fun control(args: JSONObject): JSONObject {
        val action = args.optString("action", "state").trim().lowercase()
        val observedAtDispatch = observedState()
        val refreshFromPageGeneration = observedAtDispatch.pageGeneration
        if (action !in LOCAL_ACTIONS) {
            if (bridgeState() != ChatGptWebPageAdapter.State.READY) {
                return error(action, "bridge_not_ready")
            }
            if (!observedAtDispatch.adapterCurrent) {
                return error(action, "adapter_generation_not_ready")
            }
        }
        var commandRequest: ChatGptWebObservedState.CommandRequest? = null
        fun dispatch(expectedAction: String, block: (String) -> Unit) {
            val request = beginCommand(expectedAction)
            commandRequest = request
            block(request.id)
        }
        when (action) {
            "state", "open_chatgpt_web" -> Unit
            "set_input_text", "chatgpt_set_page_input_text" -> {
                val text = args.optString("text").take(MAX_INPUT_CHARS)
                val expectedDraft = snapshot()?.draft.orEmpty().take(MAX_INPUT_CHARS)
                setInputText(text)
                dispatch("set_draft") { requestId ->
                    commands.setDraft(text, expectedDraft, requestId)
                }
            }
            "send_input", "chatgpt_send_page_input" -> dispatch("send_prompt", commands::sendInput)
            "chatgpt_invoke_control" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                ChatGptWebControlInvocationPolicy.rejection(
                    control = control,
                    userConfirmed = args.optBoolean("user_confirmed", false),
                )?.let { rejection ->
                    return error(action, rejection)
                        .put("required_argument", "user_confirmed")
                        .put("control_semantic", control.semantic)
                }
                if (control.role == "slider" && !control.supportsSliderValue) {
                    return error(action, "control_requires_official_fallback")
                }
                dispatch("invoke_ui_control") { requestId ->
                    commands.invokeControl(controlId, requestId)
                }
            }
            "chatgpt_set_control_text" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                if (!control.supportsTextEntry) return error(action, "control_not_writable")
                if (!args.has("text") || args.isNull("text")) return error(action, "missing_text")
                val text = args.optString("text").take(MAX_INPUT_CHARS)
                dispatch("set_ui_control_text") { requestId ->
                    commands.setControlText(controlId, text, requestId)
                }
            }
            "chatgpt_set_control_selected" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                val selected = args.opt("selected") as? Boolean
                    ?: return error(action, "missing_selected")
                ChatGptWebSelectedStatePolicy.rejection(control, selected)?.let { rejection ->
                    return error(action, rejection)
                }
                dispatch("set_ui_control_selected") { requestId ->
                    commands.setControlSelected(controlId, selected, requestId)
                }
            }
            "chatgpt_select_control_choice" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                if (!control.supportsChoiceSelection) return error(action, "control_choices_unavailable")
                val rawChoiceIndex = args.opt("choice_index") as? Number
                    ?: return error(action, "missing_choice_index")
                val choiceIndex = rawChoiceIndex.toInt()
                if (rawChoiceIndex.toDouble() != choiceIndex.toDouble()) {
                    return error(action, "invalid_choice_index")
                }
                if (choiceIndex !in control.choiceLabels.indices) {
                    return error(action, "invalid_choice_index")
                }
                dispatch("select_ui_control_choice") { requestId ->
                    commands.selectControlChoice(controlId, choiceIndex, requestId)
                }
            }
            "chatgpt_set_control_slider" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                val slider = control.slider
                    ?.takeIf { control.supportsSliderValue }
                    ?: return error(action, "control_slider_unavailable")
                val value = (args.opt("value") as? Number)?.toDouble()
                    ?.takeIf(Double::isFinite)
                    ?: return error(action, "missing_slider_value")
                if (value !in slider.min..slider.max) return error(action, "slider_value_out_of_range")
                dispatch("set_ui_control_slider") { requestId ->
                    commands.setControlSlider(controlId, value, requestId)
                }
            }
            "chatgpt_set_control_expanded" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                val control = uiManifest()?.controls?.firstOrNull { it.id == controlId }
                    ?: return error(action, "stale_control_id")
                if (!control.supportsExpandedState) return error(action, "control_expansion_unavailable")
                val expanded = args.opt("expanded") as? Boolean
                    ?: return error(action, "missing_expanded")
                dispatch("set_ui_control_expanded") { requestId ->
                    commands.setControlExpanded(controlId, expanded, requestId)
                }
            }
            "chatgpt_new_conversation" -> dispatch("new_conversation", commands::newConversation)
            "chatgpt_stop_generation" -> dispatch("stop_generation", commands::stopGeneration)
            "chatgpt_verify_private_stream_watchdog" -> dispatch(
                "verify_private_stream_watchdog",
                commands::verifyPrivateStreamWatchdog,
            )
            "chatgpt_copy_last_response" -> return ChatGptWebCopyAction.execute(snapshot(), copyMessage)
            "chatgpt_regenerate_response" -> {
                val current = snapshot()
                if (current?.streaming == true) return error(action, "generation_in_progress")
                if (
                    current?.capabilities?.supports(ChatGptWebCapabilityId.MESSAGE_REGENERATE) != true ||
                    current.messages.lastOrNull { it.role == "assistant" }?.state != "completed"
                ) {
                    return error(action, "regenerate_unavailable")
                }
                dispatch("regenerate_response", commands::regenerateResponse)
            }
            "chatgpt_start_dictation" -> {
                val current = snapshot()
                if (current?.dictationActive == true) return error(action, "dictation_already_active")
                if (!ChatGptDictationPolicy.isAvailable(current, uiManifest())) {
                    return error(action, "dictation_unavailable")
                }
                dispatch("start_dictation", commands::startDictation)
            }
            "chatgpt_prepare_realtime_voice" -> {
                if (inputText().isNotEmpty()) return error(action, "native_draft_not_empty")
                val expectedDraft = snapshot()?.draft?.take(MAX_INPUT_CHARS)
                    ?: return error(action, "draft_unavailable")
                if (expectedDraft.isNotEmpty()) {
                    dispatch("set_draft") { requestId ->
                        commands.setDraft("", expectedDraft, requestId)
                    }
                }
            }
            "chatgpt_start_realtime_voice" -> {
                val voiceControl = ChatGptRealtimeVoicePolicy.resolve(uiManifest())
                    ?: return error(action, "realtime_voice_unavailable")
                dispatch("invoke_ui_control") { requestId ->
                    commands.invokeControl(voiceControl.id, requestId)
                }
            }
            "chatgpt_cancel_dictation" -> {
                if (snapshot()?.dictationActive != true) return error(action, "dictation_not_active")
                dispatch("cancel_dictation", commands::cancelDictation)
            }
            "chatgpt_submit_dictation" -> {
                if (snapshot()?.dictationActive != true) return error(action, "dictation_not_active")
                dispatch("submit_dictation", commands::submitDictation)
            }
            "chatgpt_remove_attachment" -> {
                val attachmentId = args.optString("attachment_id").trim()
                if (attachmentId.isBlank() || attachmentId.length > MAX_ATTACHMENT_ID_CHARS) {
                    return error(action, "invalid_attachment_id")
                }
                val attachment = snapshot()?.attachments?.firstOrNull { it.id == attachmentId }
                    ?: return error(action, "stale_attachment_id")
                if (!attachment.removable) return error(action, "attachment_not_removable")
                dispatch("remove_attachment") { requestId ->
                    commands.removeAttachment(attachmentId, requestId)
                }
            }
            "chatgpt_refresh" -> refresh()
            "chatgpt_refresh_controls" -> dispatch("snapshot_ui_manifest", commands::refreshControls)
            "chatgpt_list_conversations" -> dispatch("list_conversations", commands::listConversations)
            "chatgpt_list_composer_options" -> {
                val section = args.optString("section").trim().lowercase()
                if (section !in COMPOSER_SECTIONS) return error(action, "invalid_section")
                dispatch(if (section == "model") "list_model_options" else "list_composer_tools") { requestId ->
                    commands.requestComposerOptions(section, requestId)
                }
            }
            "chatgpt_dismiss_composer_options" ->
                dispatch("dismiss_composer_menu", commands::dismissComposerOptions)
            "chatgpt_select_composer_option" -> {
                val section = args.optString("section").trim().lowercase()
                if (section !in COMPOSER_SECTIONS) return error(action, "invalid_section")
                val optionId = args.optString("option_id").trim()
                val options = observedState().composerSections[section].orEmpty()
                if (options.none { it.id == optionId }) return error(action, "stale_option_id")
                dispatch(if (section == "model") "select_model_option" else "select_composer_tool") { requestId ->
                    commands.selectComposerOption(section, optionId, requestId)
                }
            }
            "chatgpt_list_features" -> dispatch("list_navigation", commands::requestFeatures)
            "chatgpt_dismiss_features" -> dispatch("dismiss_navigation", commands::dismissFeatures)
            "chatgpt_select_feature" -> {
                val featureId = args.optString("feature_id").trim()
                val feature = observedState().features.firstOrNull { it.id == featureId }
                ChatGptWebProductCapabilityCatalog.selectionError(
                    feature,
                    args.optBoolean("user_confirmed", false),
                )?.let { return error(action, it) }
                dispatch("select_navigation") { requestId ->
                    commands.selectFeature(featureId, requestId)
                }
            }
            "chatgpt_get_context" -> return contextPage(args)
            "chatgpt_reveal_message" -> {
                if (mode() != ChatGptWebPresentationMode.NATIVE) {
                    return error(action, "native_view_required")
                }
                val messageId = args.optString("message_id").trim()
                if (messageId.isBlank() || messageId.length > MAX_MESSAGE_ID_CHARS) {
                    return error(action, "invalid_message_id")
                }
                val message = snapshot()?.messages?.firstOrNull { it.id == messageId }
                    ?: return error(action, "stale_message_id")
                val partIndex = if (args.has("part_index") && !args.isNull("part_index")) {
                    val raw = args.opt("part_index") as? Number
                        ?: return error(action, "invalid_part_index")
                    raw.toInt().takeIf { raw.toDouble() == it.toDouble() }
                        ?: return error(action, "invalid_part_index")
                } else {
                    null
                }
                val nativeTarget = args.optString("target")
                    .trim()
                    .lowercase()
                    .ifBlank { ChatGptNativeMessageRevealTarget.MESSAGE }
                if (nativeTarget !in ChatGptNativeMessageRevealTarget.ALL) {
                    return error(action, "invalid_reveal_target")
                }
                if (partIndex != null && partIndex !in message.parts.indices) {
                    return error(action, "invalid_part_index")
                }
                if (partIndex != null && nativeTarget != ChatGptNativeMessageRevealTarget.MESSAGE) {
                    return error(action, "part_target_conflict")
                }
                if (!revealMessage(messageId, partIndex, nativeTarget)) {
                    return error(action, "message_not_rendered")
                }
            }
            "chatgpt_find_controls" -> return controlsPage(args)
            "chatgpt_get_conversations" -> return conversationsPage(args)
            "chatgpt_get_navigation" -> return navigationPage(args)
            "chatgpt_get_capability_matrix" -> return ChatGptWebCapabilityMatrix.build(
                snapshot().takeIf { observedAtDispatch.adapterCurrent },
                uiManifest().takeIf { observedAtDispatch.adapterCurrent },
                bridgeState(),
                mode(),
                observedAtDispatch,
                verificationEvidence(),
            )
            "chatgpt_record_verification_cases" -> {
                return when (val result = ChatGptWebVerificationEvidenceActions.record(args, snapshot()?.authenticated == true, recordVerificationCases)) {
                    is ChatGptWebVerificationEvidenceActions.Result.Success -> result.response
                    is ChatGptWebVerificationEvidenceActions.Result.Error -> error(action, result.code)
                }
            }
            "chatgpt_open_conversation" -> {
                val path = ChatGptWebConversationPath.normalize(args.optString("conversation_path"))
                    ?: return error(action, "invalid_conversation_path")
                dispatch("open_conversation") { requestId ->
                    commands.openConversation(path, requestId)
                }
            }
            "chatgpt_select_view" -> {
                val next = when (args.optString("view_mode").lowercase()) {
                    "login", "quick" -> ChatGptWebPresentationMode.QUICK
                    "official", "web" -> ChatGptWebPresentationMode.WEB
                    "native", "yilong" -> ChatGptWebPresentationMode.NATIVE
                    "skin", "web_skin" -> ChatGptWebPresentationMode.SKIN
                    else -> return error(action, "invalid_view_mode")
                }
                selectMode(next)
            }
            else -> return error(action, "unsupported_action")
        }
        return uiState()
            .put("control_ok", true)
            .put("action", action)
            .apply {
                commandRequest?.let { started ->
                    val current = observedState().commandRequests
                        .lastOrNull { it.id == started.id } ?: started
                    put(
                        "command_status",
                        if (current.status == ChatGptWebObservedState.CommandRequest.PENDING) {
                            "dispatched"
                        } else {
                            current.status
                        },
                    )
                    put("command_receipt", ChatGptWebCommandReceipts.requestJson(current))
                    put("poll_hint", "按 request_id 读取 ui_state.command_requests 确认官网命令结果")
                }
                if (action == "chatgpt_refresh") {
                    put("command_status", "dispatched")
                    put("refresh_from_page_generation", refreshFromPageGeneration)
                    put("completion_signal", "adapter_generation")
                    put(
                        "poll_hint",
                        "等待 ui_state.page_generation 大于 refresh_from_page_generation，且 adapter_current=true",
                    )
                }
            }
    }

    private fun contextPage(args: JSONObject): JSONObject {
        val current = snapshot() ?: return error("chatgpt_get_context", "conversation_unavailable")
        val limit = args.optInt("message_limit", DEFAULT_CONTEXT_PAGE_SIZE)
            .coerceIn(1, MAX_CONTEXT_PAGE_SIZE)
        val result = ChatGptWebContextPager.page(
            snapshot = current,
            cursor = args.optString("message_cursor").take(MAX_CONTEXT_CURSOR_CHARS),
            requestedOffset = args.optInt("message_offset", current.messageWindowStart),
            requestedLimit = limit,
            maxLimit = MAX_CONTEXT_PAGE_SIZE,
        )
        if (result is ChatGptWebContextPager.Result.Failure) {
            return error("chatgpt_get_context", result.code)
                .put("schema", ChatGptWebContextPager.SCHEMA)
                .put("current_context_revision", result.currentRevision)
                .put("message_count", result.observedMessageCount)
                .put("available_message_count", result.messageWindowEnd - result.messageWindowStart)
                .put("message_window_start", result.messageWindowStart)
                .put("message_window_end", result.messageWindowEnd)
                .put("history_truncated", result.messageWindowStart > 0)
                .put("retry_from_message_offset", result.messageWindowStart)
        }
        val page = (result as ChatGptWebContextPager.Result.Success).page
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_context")
            .put("schema", ChatGptWebContextPager.SCHEMA)
            .put("conversation_title", current.title)
            .put("conversation_url", current.url)
            .put("message_count", current.observedMessageCount)
            .put("available_message_count", current.messages.size)
            .put("message_window_start", current.messageWindowStart)
            .put("message_window_end", current.messageWindowStart + current.messages.size)
            .put("history_truncated", current.messageWindowStart > 0)
            .put("has_more_before", page.hasMoreBefore)
            .put(
                "context_complete",
                current.messageWindowStart == 0 && current.messages.size >= current.observedMessageCount,
            )
            .put("context_revision", page.revision)
            .put("context_streaming", current.streaming)
            .put("cursor_stable", current.streaming.not())
            .put("message_cursor", page.cursor)
            .put("next_message_cursor", page.nextCursor ?: JSONObject.NULL)
            .put("message_offset", page.offset)
            .put("message_limit", page.limit)
            .put("next_message_offset", page.nextOffset)
            .put("has_more", page.hasMore)
            .put("messages", ChatGptWebMessageJson.encode(
                page.messages,
                page.offset,
                MAX_CONTEXT_MESSAGE_CHARS,
            ))
    }

    private fun controlsPage(args: JSONObject): JSONObject {
        val manifest = uiManifest() ?: return error("chatgpt_find_controls", "manifest_unavailable")
        val query = args.optString("query").trim()
        val semantic = args.optString("semantic").trim().lowercase()
        val region = args.optString("region").trim().lowercase()
        val contextId = args.optString("context_id").trim()
        val matches = manifest.controls.filter { control ->
            (query.isBlank() || control.label.contains(query, ignoreCase = true)) &&
                (semantic.isBlank() || control.semantic == semantic) &&
                (region.isBlank() || control.region == region) &&
                (contextId.isBlank() || control.contextId == contextId)
        }
        val presentations = ChatGptNativeControlPresentation.describe(manifest.controls)
        val page = page(args, matches.size, DEFAULT_CONTROL_PAGE_SIZE, MAX_CONTROL_PAGE_SIZE)
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_find_controls")
            .put("query", query)
            .put("semantic", semantic)
            .put("region", region)
            .put("context_id", contextId)
            .put("match_count", matches.size)
            .put("manifest_control_count", manifest.controls.size)
            .put("discovered_control_count", manifest.discoveredControlCount)
            .put("controls_truncated", manifest.controlsTruncated)
            .put("offset", page.offset)
            .put("limit", page.limit)
            .put("next_offset", page.nextOffset)
            .put("has_more", page.hasMore)
            .put("controls", JSONArray().apply {
                matches.drop(page.offset).take(page.limit).forEach { control ->
                    put(controlJson(control, presentations[control.id]))
                }
            })
    }

    private fun conversationsPage(args: JSONObject): JSONObject {
        val observed = observedState()
        val query = args.optString("query").trim()
        val matches = observed.conversations.filter {
            query.isBlank() || it.title.contains(query, ignoreCase = true)
        }
        val page = page(args, matches.size, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE)
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_conversations")
            .put("query", query)
            .put("cached_at_ms", observed.updatedAtMs)
            .put("source_count", observed.conversations.size)
            .put("project_count", observed.projects.size)
            .put("projects", ChatGptWebProjectJson.encode(observed.projects))
            .put("source", observed.conversationCollection.source)
            .put("stale", observed.conversationCollection.stale)
            .put("collection", ChatGptWebConversationCollectionJson.encode(observed.conversationCollection))
            .put("match_count", matches.size)
            .put("offset", page.offset)
            .put("limit", page.limit)
            .put("next_offset", page.nextOffset)
            .put("has_more", page.hasMore)
            .put("conversations", JSONArray().apply {
                matches.drop(page.offset).take(page.limit).forEach { conversation ->
                    put(JSONObject()
                        .put("id", conversation.id)
                        .put("title", conversation.title)
                        .put("path", conversation.path)
                        .put("active", conversation.active)
                        .put("group_label", conversation.groupLabel)
                        .put("project_id", conversation.projectId ?: JSONObject.NULL)
                        .put("project_title", conversation.projectTitle ?: JSONObject.NULL)
                        .put("project_path", conversation.projectPath ?: JSONObject.NULL)
                        .put("activity_dates", JSONArray(conversation.activityDates.sorted()))
                        .put("native_action", "chatgpt_open_conversation")
                        .put(
                            "native_adb_content_description",
                            ChatGptNativeNavigationSelector.conversation(conversation),
                        )
                    )
                }
            })
    }

    private fun navigationPage(args: JSONObject): JSONObject {
        val observed = observedState()
        val section = args.optString("section").trim().lowercase()
        val optionSections = observed.composerSections
            .filterKeys { section.isBlank() || it == section }
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_navigation")
            .put("schema", NAVIGATION_SCHEMA)
            .put("native_selector_schema", ChatGptNativeNavigationSelector.SCHEMA)
            .put("cached_at_ms", observed.updatedAtMs)
            .put("features", JSONArray().apply {
                observed.features.forEach { feature ->
                    put(ChatGptWebProductCapabilityCatalog.navigationJson(feature))
                }
            })
            .put("composer_sections", JSONObject().apply {
                optionSections.forEach { (name, options) ->
                    put(name, JSONArray().apply {
                        options.forEach { option ->
                            put(JSONObject()
                                .put("id", option.id)
                                .put("label", option.label)
                                .put("kind", option.kind)
                                .put("semantic", option.semantic)
                                .put("selected", option.selected)
                                .put("opens_submenu", option.opensSubmenu)
                                .put("native_action", "chatgpt_select_composer_option")
                                .put(
                                    "native_adb_content_description",
                                    ChatGptNativeNavigationSelector.composerOption(name, option),
                                )
                            )
                        }
                    })
                }
            })
    }

    private fun conversationJson(value: ChatGptWebSnapshot?): Any {
        if (value == null) return JSONObject.NULL
        val exportedMessages = value.messages.takeLast(MAX_MESSAGES)
        val exportedStart = value.messageWindowStart + value.messages.size - exportedMessages.size
        val windowEnd = value.messageWindowStart + value.messages.size
        return JSONObject()
            .put("schema", CONVERSATION_SUMMARY_SCHEMA)
            .put("title", value.title)
            .put("url", value.url)
            .put("current_model", value.currentModel)
            .put("message_count", value.observedMessageCount)
            .put("available_message_count", value.messages.size)
            .put("message_window_start", value.messageWindowStart)
            .put("message_window_end", windowEnd)
            .put("history_truncated", value.messageWindowStart > 0)
            .put(
                "context_complete",
                value.messageWindowStart == 0 && windowEnd >= value.observedMessageCount,
            )
            .put("exported_message_count", exportedMessages.size)
            .put("exported_message_offset", exportedStart)
            .put(
                "messages_truncated",
                exportedMessages.size < value.messages.size || windowEnd < value.observedMessageCount,
            )
            .put("context_action", "chatgpt_get_context")
            .put("messages", ChatGptWebMessageJson.encode(
                exportedMessages,
                exportedStart,
                MAX_MESSAGE_CHARS,
            ))
            .put("attachments", JSONArray().apply {
                value.attachments.forEach { attachment ->
                    put(JSONObject()
                        .put("id", attachment.id)
                        .put("name", attachment.name)
                        .put("state", attachment.state)
                    )
                }
            })
    }

    private fun navigationSummary(value: ChatGptWebObservedState.Snapshot): JSONObject = JSONObject()
        .put("conversation_count", value.conversations.size)
        .put(
            "conversation_collection",
            ChatGptWebConversationCollectionJson.encode(value.conversationCollection),
        )
        .put("feature_count", value.features.size)
        .put("composer_sections", JSONArray(value.composerSections.keys.sorted()))
        .put("cached_at_ms", value.updatedAtMs)

    private fun manifestJson(value: ChatGptWebUiManifest?): Any {
        if (value == null) return JSONObject.NULL
        val presentations = ChatGptNativeControlPresentation.describe(value.controls)
        return JSONObject()
            .put("version", value.version)
            .put("page_kind", value.pageKind)
            .put("title", value.title)
            .put("compatibility", value.compatibility)
            .put("control_count", value.controls.size)
            .put("discovered_control_count", value.discoveredControlCount)
            .put("controls_truncated", value.controlsTruncated)
            .put("generic_control_count", value.controls.count { it.semantic == "action" })
            .put("message_control_count", value.controls.count { it.region == ChatGptWebUiRegion.MESSAGE })
            .put("web_position_count", value.controls.count { it.webXRatio != null && it.webYRatio != null })
            .put("controls", JSONArray().apply {
                value.controls.forEach { control ->
                    put(controlJson(control, presentations[control.id]))
                }
            })
    }

    private fun controlJson(
        control: ChatGptWebUiControl,
        presentation: ChatGptNativeControlPresentation.Coverage?,
    ): JSONObject {
        val invocationRisk = ChatGptWebControlInvocationPolicy.risk(control)
        return JSONObject()
            .put("control_id", control.id)
            .put("semantic", control.semantic)
            .put("label", control.label)
            .put("region", control.region)
            .put("role", control.role)
            .put("enabled", control.enabled)
            .put("invocation_risk", invocationRisk.wireName)
            .put(
                "requires_user_confirmation",
                invocationRisk == ChatGptWebControlInvocationPolicy.Risk.USER_CONFIRMATION,
            )
            .put("selected", control.selected)
            .put("input_kind", control.inputKind ?: JSONObject.NULL)
            .put("writable", control.writable)
            .put("state_settable", control.supportsSelectedState)
            .put("expanded", control.expanded ?: JSONObject.NULL)
            .put("expandable", control.supportsExpandedState)
            .put("choice_labels", JSONArray(control.choiceLabels))
            .put("selected_choice_index", control.selectedChoiceIndex ?: JSONObject.NULL)
            .put(
                "slider",
                control.slider?.let { slider ->
                    JSONObject()
                        .put("min", slider.min)
                        .put("max", slider.max)
                        .put("step", slider.step)
                        .put("value", slider.value)
                        .put("native_input_content_description", ChatGptNativeSliderControlDialog.sliderSelector(control.id))
                        .put("native_value_content_description", ChatGptNativeSliderControlDialog.valueSelector(control.id))
                        .put("native_commit_content_description", ChatGptNativeSliderControlDialog.commitSelector(control.id))
                } ?: JSONObject.NULL,
            )
            .put("context_id", control.contextId ?: JSONObject.NULL)
            .put("in_viewport", control.inViewport)
            .put("web_x_ratio", control.webXRatio ?: JSONObject.NULL)
            .put("web_y_ratio", control.webYRatio ?: JSONObject.NULL)
            .put("adb_content_description", control.accessibilityLabel)
            .put("native_presentation", presentation?.kind?.wireName ?: "official_fallback")
            .put(
                "native_adb_content_description",
                presentation?.nativeSelector ?: JSONObject.NULL,
            )
            .put(
                "native_trigger_content_description",
                presentation?.nativeTriggerSelector ?: JSONObject.NULL,
            )
            .put(
                "native_value_input_content_description",
                if (control.supportsTextEntry) {
                    ChatGptNativeFormControlDialog.inputSelector(control.id)
                } else {
                    JSONObject.NULL
                },
            )
            .put(
                "native_value_commit_content_description",
                if (control.supportsTextEntry) {
                    ChatGptNativeFormControlDialog.commitSelector(control.id)
                } else {
                    JSONObject.NULL
                },
            )
            .put(
                "native_choice_content_descriptions",
                JSONArray().apply {
                    if (control.supportsChoiceSelection) {
                        control.choiceLabels.indices.forEach { index ->
                            put(ChatGptNativeChoiceControlDialog.choiceSelector(control.id, index))
                        }
                    }
                },
            )
    }

    private fun page(
        args: JSONObject,
        total: Int,
        defaultLimit: Int,
        maxLimit: Int,
    ): Page {
        val offset = args.optInt("offset", 0).coerceIn(0, total)
        val limit = args.optInt("limit", defaultLimit).coerceIn(1, maxLimit)
        val nextOffset = (offset + limit).coerceAtMost(total)
        return Page(offset, limit, nextOffset, nextOffset < total)
    }

    private fun error(action: String, code: String): JSONObject = uiState()
        .put("control_ok", false)
        .put("action", action)
        .put("error", code)

    private companion object {
        data class Page(
            val offset: Int,
            val limit: Int,
            val nextOffset: Int,
            val hasMore: Boolean,
        )

        const val MAX_MESSAGES = 50
        const val MAX_MESSAGE_CHARS = 30_000
        const val MAX_CONTEXT_MESSAGE_CHARS = 40_000
        const val MAX_CONTEXT_CURSOR_CHARS = 80
        const val MAX_MESSAGE_ID_CHARS = 200
        const val MAX_INPUT_CHARS = 20_000
        const val MAX_ATTACHMENT_ID_CHARS = 96
        const val DEFAULT_CONTEXT_PAGE_SIZE = 20
        const val MAX_CONTEXT_PAGE_SIZE = 40
        const val DEFAULT_CONTROL_PAGE_SIZE = 30
        const val MAX_CONTROL_PAGE_SIZE = 80
        const val DEFAULT_LIST_PAGE_SIZE = 30
        const val MAX_LIST_PAGE_SIZE = 50
        const val NAVIGATION_SCHEMA = "elon.chatgpt_web.navigation.v2"
        const val CONVERSATION_SUMMARY_SCHEMA = "elon.chatgpt_web.conversation_summary.v2"
        val CONTROL_ID = Regex("control_[a-z0-9_]{1,63}")
        val COMPOSER_SECTIONS = setOf("model", "tools")
        val LOCAL_ACTIONS = setOf(
            "state",
            "open_chatgpt_web",
            "chatgpt_refresh",
            "chatgpt_get_capability_matrix",
            "chatgpt_select_view",
        )
    }
}

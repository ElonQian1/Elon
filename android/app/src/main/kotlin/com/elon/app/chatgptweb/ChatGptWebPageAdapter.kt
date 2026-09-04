package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.BuildConfig
import com.elon.app.WebBridgeDocumentSession
import java.nio.charset.StandardCharsets
import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebPageAdapter(
    context: Context,
    private val webView: WebView,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onStateChanged: (State) -> Unit,
    private val onDocumentChanged: (WebBridgeDocumentSession.Snapshot) -> Unit = {},
    private val onWebExecutionRequested: () -> Unit = {},
) : ChatGptOfficialPageSendCommandPort {
    enum class State {
        WEB_ONLY,
        CONNECTING,
        READY,
        UNSUPPORTED,
    }

    private val privateEarlyTransportEnabled =
            BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED ||
            BuildConfig.CHATGPT_PRIVATE_CONVERSATION_MUTATIONS_ENABLED ||
            BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED ||
            BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED ||
            BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED ||
            BuildConfig.CHATGPT_PRIVATE_READ_ALOUD_ENABLED
    private val privateAuthContextScript =
        context.assets.open(PRIVATE_AUTH_CONTEXT_ASSET).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }

    private val adapterScript = """
        (function () {
            window.__elonChatGptAdapterTargetVersion = $ADAPTER_VERSION;
            window.__elonChatGptPrivateResearchEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED};
            window.__elonChatGptPrivateVoiceNativeRtcEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED};
            window.__elonChatGptPrivateConversationPrefetchEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED};
            window.__elonChatGptPrivateConversationMutationsEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_CONVERSATION_MUTATIONS_ENABLED};
            window.__elonChatGptPrivateStreamObserverEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED};
            window.__elonChatGptPrivateTextTransactionsEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED};
            window.__elonChatGptPrivateDictationEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED};
            window.__elonChatGptPrivateReadAloudEnabled =
                ${BuildConfig.CHATGPT_PRIVATE_READ_ALOUD_ENABLED};
            if (!/^doc_[a-z0-9_]{3,80}$/.test(String(window.__elonChatGptDocumentToken || ""))) {
                window.__elonChatGptDocumentToken =
                    "doc_android_" + Math.random().toString(36).slice(2) + Date.now().toString(36);
            }
        })();
    """.trimIndent() + "\n" + privateAuthContextScript + "\n" +
        ADAPTER_ASSETS.joinToString("\n") { asset ->
            context.assets.open(asset).use { input ->
                input.reader(StandardCharsets.UTF_8).readText()
            }
        }
    private val privateEarlyTapScript = """
        window.__elonChatGptPrivateAuthContextEnabled =
            ${BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED ||
                BuildConfig.CHATGPT_PRIVATE_CONVERSATION_MUTATIONS_ENABLED ||
                BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED ||
                BuildConfig.CHATGPT_PRIVATE_READ_ALOUD_ENABLED};
        window.__elonChatGptPrivateStreamObserverEnabled =
            ${BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED};
        window.__elonChatGptPrivateTextTransactionsEnabled =
            ${BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED};
    """.trimIndent() + "\n" + privateAuthContextScript + "\n" + listOf(
        PRIVATE_FETCH_TAP_ASSET,
        PRIVATE_TEXT_TRANSACTION_POLICY_ASSET,
        PRIVATE_TEXT_TRANSACTION_RELAY_ASSET,
        PRIVATE_SOCKET_TAP_ASSET,
    )
        .joinToString("\n") { asset -> context.assets.open(asset).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }
    }
    private val privateRealtimeVoiceResearchScript = """
        window.__elonChatGptAdapterTargetVersion = $ADAPTER_VERSION;
        window.__elonChatGptPrivateResearchEnabled =
            ${BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED};
        window.__elonChatGptPrivateVoiceNativeRtcEnabled =
            ${BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED};
        if (!/^doc_[a-z0-9_]{3,80}$/.test(String(window.__elonChatGptDocumentToken || ""))) {
            window.__elonChatGptDocumentToken =
                "doc_android_" + Math.random().toString(36).slice(2) + Date.now().toString(36);
        }
    """.trimIndent() + "\n" + listOf(
        PRIVATE_REALTIME_VOICE_RELAY_ASSET,
        PRIVATE_REALTIME_DATA_CHANNEL_RESEARCH_ASSET,
        PRIVATE_REALTIME_VOICE_RESEARCH_ASSET,
    ).joinToString("\n") { asset ->
        context.assets.open(asset).use { input -> input.reader(StandardCharsets.UTF_8).readText() }
    }
    private val privateConversationDirectoryScript =
        context.assets.open(PRIVATE_CONVERSATION_DIRECTORY_ASSET).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }
    private val attachmentTransportObserverScript = """
        window.__elonChatGptAdapterTargetVersion = $ADAPTER_VERSION;
    """.trimIndent() + "\n" +
        context.assets.open(ATTACHMENT_TRANSPORT_OBSERVER_ASSET).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }
    private val mainHandler = Handler(Looper.getMainLooper())
    private val documentSession = WebBridgeDocumentSession()
    private val handshake = ChatGptWebBridgeHandshake(
        schedule = { delayMs, action -> mainHandler.postDelayed({ action() }, delayMs) },
        injectAndRequestSnapshot = ::injectAndRequestSnapshot,
    )
    private var listenerInstalled = false
    private var skinEnabled = false

    fun install() {
        if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
            onStateChanged(State.UNSUPPORTED)
            return
        }
        WebViewCompat.addWebMessageListener(
            webView,
            BRIDGE_OBJECT,
            setOf(ALLOWED_ORIGIN),
        ) { _, message, sourceOrigin, isMainFrame, _ ->
            if (!isMainFrame || !isAllowedOrigin(sourceOrigin)) return@addWebMessageListener
            val payload = message.data ?: return@addWebMessageListener
            val parsed = ChatGptWebProtocol.parseMessage(payload, ADAPTER_VERSION)
                ?: return@addWebMessageListener
            val token = parsed.documentToken ?: return@addWebMessageListener
            val wasCurrent = documentSession.snapshot().adapterCurrent
            val document = documentSession.accept(token) ?: return@addWebMessageListener
            if (!wasCurrent) onDocumentChanged(document)
            if (parsed.event.completesHandshake()) handshake.acknowledge()
            onEvent(parsed.event)
        }
        if (
            (
                BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED ||
                    BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED
            ) &&
            WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
        ) {
            WebViewCompat.addDocumentStartJavaScript(
                webView,
                privateRealtimeVoiceResearchScript,
                setOf(ALLOWED_ORIGIN),
            )
        }
        if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
            WebViewCompat.addDocumentStartJavaScript(
                webView,
                attachmentTransportObserverScript,
                setOf(ALLOWED_ORIGIN),
            )
        }
        if (
            privateEarlyTransportEnabled &&
            WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
        ) {
            WebViewCompat.addDocumentStartJavaScript(
                webView,
                privateEarlyTapScript,
                setOf(ALLOWED_ORIGIN),
            )
        }
        if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
            WebViewCompat.addDocumentStartJavaScript(
                webView,
                privateConversationDirectoryScript,
                setOf(ALLOWED_ORIGIN),
            )
        }
        listenerInstalled = true
        onStateChanged(State.WEB_ONLY)
    }

    fun onPageReady(url: String) {
        val enhancedModeSupported = ChatGptWebNavigationPolicy.supportsEnhancedMode(url)
        if (
            enhancedModeSupported &&
            listenerInstalled &&
            documentSession.snapshot().pageGeneration == 0L
        ) {
            onDocumentChanged(documentSession.ensurePage())
        }
        val document = documentSession.snapshot()
        onStateChanged(ChatGptWebBridgeReadinessPolicy.stateAfterPageReady(
            listenerInstalled = listenerInstalled,
            enhancedModeSupported = enhancedModeSupported,
            document = document,
        ))
        if (!enhancedModeSupported || !listenerInstalled) return
        handshake.start()
    }

    fun onPageStarted(url: String) {
        handshake.cancel()
        onDocumentChanged(documentSession.beginPage())
        val state = if (
            listenerInstalled && ChatGptWebNavigationPolicy.supportsEnhancedMode(url)
        ) {
            State.CONNECTING
        } else {
            State.WEB_ONLY
        }
        onStateChanged(state)
    }

    fun onHostResumed(url: String?) {
        if (listenerInstalled && ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) {
            if (documentSession.snapshot().pageGeneration == 0L) {
                onDocumentChanged(documentSession.ensurePage())
            }
            handshake.start()
        }
    }

    fun onHostPaused() {
        handshake.cancel()
        mainHandler.removeCallbacksAndMessages(null)
        webView.evaluateJavascript(
            "try{var b=window.__elonChatGptBridge;if(b&&typeof b.dispose==='function')b.dispose();}catch(_){}" +
                "try{delete window.__elonChatGptBridge;}catch(_){window.__elonChatGptBridge=undefined;}",
            null,
        )
    }

    override fun sendPrompt(
        prompt: String,
        expectedDraft: String,
        requestId: String?,
        allowPrivateTextTransaction: Boolean,
    ) = runCommand(
        action = "send_prompt",
        value = prompt.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
        requestId = requestId,
        allowPrivateTextTransaction = allowPrivateTextTransaction,
    )

    fun setDraft(value: String, expectedDraft: String, requestId: String) = runCommand(
        action = "set_draft",
        value = value.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
        requestId = requestId,
    )

    fun stopGeneration() = runCommand("stop_generation")

    fun stopGeneration(requestId: String) = runCommand("stop_generation", requestId = requestId)

    fun verifyPrivateStreamWatchdog(requestId: String) = runCommand(
        "verify_private_stream_watchdog",
        requestId = requestId,
    )

    fun regenerateResponse() = runCommand("regenerate_response")

    fun regenerateResponse(requestId: String) = runCommand(
        "regenerate_response",
        requestId = requestId,
    )

    fun togglePrivateReadAloud(contextId: String, requestId: String) = runCommand(
        action = "toggle_private_read_aloud",
        value = contextId.take(MAX_MESSAGE_CONTEXT_ID_LENGTH),
        requestId = requestId,
    )

    fun setConversationPinned(path: String, pinned: Boolean, requestId: String) = runCommand(
        action = "set_conversation_pinned",
        value = path.take(MAX_CONVERSATION_PATH_LENGTH),
        requestId = requestId,
        selected = pinned,
    )

    fun startNewConversation() = runCommand("new_conversation")

    fun startNewConversation(requestId: String) = runCommand("new_conversation", requestId = requestId)

    fun listConversations(
        projectHints: List<ChatGptWebProject> = emptyList(),
        scopeProjectId: String? = null,
    ) = runCommand(
        action = "list_conversations",
        projectHints = projectHints,
        projectScopeId = scopeProjectId,
    )

    fun listConversations(requestId: String) = runCommand("list_conversations", requestId = requestId)

    fun cancelConversationDirectoryWork() = runCommand("cancel_conversation_directory")

    fun probeConversationProject(path: String, projectId: String): Boolean {
        val normalizedPath = ChatGptWebConversationPath.normalize(path) ?: return false
        val normalizedProjectId = ChatGptWebConversationPath.canonicalProjectId(projectId)
            ?: return false
        runCommand(
            action = "probe_conversation_project",
            value = normalizedPath,
            projectScopeId = normalizedProjectId,
        )
        return true
    }

    fun openConversation(path: String) = runCommand(
        action = "open_conversation",
        value = path.take(MAX_CONVERSATION_PATH_LENGTH),
    )

    fun openConversation(path: String, requestId: String) = runCommand(
        action = "open_conversation",
        value = path.take(MAX_CONVERSATION_PATH_LENGTH),
        requestId = requestId,
    )

    fun openProject(path: String) = runCommand(
        action = "open_project",
        value = path.take(MAX_CONVERSATION_PATH_LENGTH),
    )

    fun startGoogleLogin() = runCommand("start_google_login")

    fun listModelOptions(requestId: String? = null) = runCommand("list_model_options", requestId = requestId)

    fun listComposerTools(requestId: String? = null) = runCommand("list_composer_tools", requestId = requestId)

    fun collectModelOptions() = runCommand("collect_model_options")

    fun collectComposerTools() = runCommand("collect_composer_tools")

    fun dismissComposerMenu(requestId: String? = null) =
        runCommand("dismiss_composer_menu", requestId = requestId)

    fun selectModelOption(id: String, requestId: String? = null) = runCommand(
        "select_model_option",
        id.take(MAX_OPTION_ID_LENGTH),
        requestId = requestId,
    )

    fun selectComposerTool(id: String, requestId: String? = null) = runCommand(
        "select_composer_tool",
        id.take(MAX_OPTION_ID_LENGTH),
        requestId = requestId,
    )

    fun requestAttachmentUpload() = runCommand("request_attachment_upload")

    fun startDictation() {
        ChatGptWebPrivateResearchEventRecorder.beginVoiceWindow()
        runCommand("start_dictation")
    }

    fun startDictation(
        nativeDraft: String,
        expectedOfficialDraft: String,
        requestId: String,
    ) {
        ChatGptWebPrivateResearchEventRecorder.beginVoiceWindow()
        runCommand(
            action = "start_dictation",
            value = nativeDraft.take(MAX_PROMPT_LENGTH),
            expectedDraft = expectedOfficialDraft.take(MAX_PROMPT_LENGTH),
            requestId = requestId,
        )
    }

    fun cancelDictation() = runCommand("cancel_dictation")

    fun cancelDictation(requestId: String) = runCommand("cancel_dictation", requestId = requestId)

    fun submitDictation() = runCommand("submit_dictation")

    fun submitDictation(requestId: String) = runCommand("submit_dictation", requestId = requestId)

    fun startPrivateDictation(nativeDraft: String, expectedOfficialDraft: String) = runCommand(
        action = "private_start_dictation",
        value = nativeDraft.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedOfficialDraft.take(MAX_PROMPT_LENGTH),
    )

    fun cancelPrivateDictation() = runCommand("private_cancel_dictation")

    fun submitPrivateDictation() = runCommand("private_submit_dictation")

    fun removeAttachment(id: String) = runCommand("remove_attachment", id.take(MAX_OPTION_ID_LENGTH))

    fun removeAttachment(id: String, requestId: String) = runCommand(
        "remove_attachment",
        id.take(MAX_OPTION_ID_LENGTH),
        requestId = requestId,
    )

    fun listFeatures() = runCommand("list_navigation")

    fun listFeatures(requestId: String) = runCommand("list_navigation", requestId = requestId)

    fun collectFeatures() = runCommand("collect_navigation")

    fun selectFeature(id: String) = runCommand("select_navigation", id.take(MAX_OPTION_ID_LENGTH))

    fun selectFeature(id: String, requestId: String) = runCommand(
        "select_navigation",
        id.take(MAX_OPTION_ID_LENGTH),
        requestId = requestId,
    )

    fun dismissFeatures() = runCommand("dismiss_navigation")

    fun dismissFeatures(requestId: String) = runCommand("dismiss_navigation", requestId = requestId)

    fun requestUiManifest() = runCommand("snapshot_ui_manifest")

    fun requestUiManifest(requestId: String) = runCommand(
        "snapshot_ui_manifest",
        requestId = requestId,
    )

    fun invokeUiControl(id: String, requestId: String? = null) = runCommand(
        "invoke_ui_control",
        id.take(MAX_UI_CONTROL_ID_LENGTH),
        requestId = requestId,
    )

    fun invokeUiControlAfterTouchMiss(id: String, requestId: String) = runCommand(
        "invoke_ui_control_after_touch_miss",
        id.take(MAX_UI_CONTROL_ID_LENGTH),
        requestId = requestId,
    )

    fun revealProjectChoice(label: String, requestId: String) = runCommand(
        action = "reveal_project_choice",
        value = label.take(MAX_PROJECT_TITLE_LENGTH),
        requestId = requestId,
    )

    fun setUiControlText(id: String, text: String, requestId: String? = null) = runCommand(
        action = "set_ui_control_text",
        value = text.take(MAX_PROMPT_LENGTH),
        requestId = requestId,
        controlId = id.take(MAX_UI_CONTROL_ID_LENGTH),
    )

    fun setUiControlSelected(id: String, selected: Boolean, requestId: String? = null) = runCommand(
        action = "set_ui_control_selected",
        requestId = requestId,
        controlId = id.take(MAX_UI_CONTROL_ID_LENGTH),
        selected = selected,
    )

    fun selectUiControlChoice(id: String, choiceIndex: Int, requestId: String? = null) = runCommand(
        action = "select_ui_control_choice",
        requestId = requestId,
        controlId = id.take(MAX_UI_CONTROL_ID_LENGTH),
        choiceIndex = choiceIndex,
    )

    fun setUiControlSlider(id: String, value: Double, requestId: String? = null) = runCommand(
        action = "set_ui_control_slider",
        requestId = requestId,
        controlId = id.take(MAX_UI_CONTROL_ID_LENGTH),
        numericValue = value,
    )

    fun setUiControlExpanded(id: String, expanded: Boolean, requestId: String? = null) = runCommand(
        action = "set_ui_control_expanded",
        requestId = requestId,
        controlId = id.take(MAX_UI_CONTROL_ID_LENGTH),
        expanded = expanded,
    )

    fun setSkinMode(enabled: Boolean) {
        skinEnabled = enabled
        runCommand(
            action = "set_skin_mode",
            selected = enabled,
        )
    }

    override fun requestSnapshot() = runCommand("snapshot")

    fun requestImageAsset(handle: String) = runCommand(
        action = "request_image_asset",
        value = handle.take(MAX_IMAGE_ASSET_HANDLE_LENGTH),
    )

    fun requestConversationRefresh() = runCommand("refresh_current_conversation")

    fun markReady() {
        if (documentSession.snapshot().adapterCurrent) onStateChanged(State.READY)
    }

    fun markLoginRequired() = onStateChanged(State.WEB_ONLY)

    fun dispose() {
        onHostPaused()
        if (listenerInstalled && WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
            WebViewCompat.removeWebMessageListener(webView, BRIDGE_OBJECT)
        }
        listenerInstalled = false
    }

    private fun injectAndRequestSnapshot() {
        if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) return
        val document = documentSession.ensurePage()
        val tokenSetup = "window.__elonChatGptDocumentToken=${JSONObject.quote(document.documentToken)};" +
            "window.__elonChatGptAdapterTargetVersion=$ADAPTER_VERSION;"
        webView.evaluateJavascript("$tokenSetup\n$adapterScript") {
            if (listenerInstalled && ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
                setSkinMode(skinEnabled)
                if (!skinEnabled) requestSnapshot()
            }
        }
    }

    private fun ChatGptWebEvent.completesHandshake(): Boolean = when (this) {
        is ChatGptWebEvent.AdapterReady -> true
        is ChatGptWebEvent.Snapshot -> value.authenticated || value.composerReady || value.dictationActive
        is ChatGptWebEvent.ConversationList,
        is ChatGptWebEvent.ComposerControls,
        is ChatGptWebEvent.FeatureNavigation,
        is ChatGptWebEvent.UiManifest,
        is ChatGptWebEvent.WebTouchRequest,
        is ChatGptWebEvent.AttachmentTransport,
        is ChatGptWebEvent.ImageAsset,
        is ChatGptWebEvent.ImageGallerySnapshot,
        is ChatGptWebEvent.CommandResult -> true
    }

    private fun runCommand(
        action: String,
        value: String? = null,
        expectedDraft: String? = null,
        requestId: String? = null,
        controlId: String? = null,
        selected: Boolean? = null,
        choiceIndex: Int? = null,
        numericValue: Double? = null,
        expanded: Boolean? = null,
        allowPrivateTextTransaction: Boolean? = null,
        projectHints: List<ChatGptWebProject> = emptyList(),
        projectScopeId: String? = null,
    ) {
        if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) return
        onWebExecutionRequested()
        val command = JSONObject()
            .put("action", action)
            .put("documentToken", documentSession.snapshot().documentToken)
            .apply {
                if (value != null) put("value", value)
                if (expectedDraft != null) put("expectedDraft", expectedDraft)
                if (requestId != null && REQUEST_ID.matches(requestId)) put("requestId", requestId)
                if (controlId != null) put("controlId", controlId)
                if (selected != null) put("selected", selected)
                if (choiceIndex != null) put("choiceIndex", choiceIndex)
                if (numericValue != null && numericValue.isFinite()) put("numericValue", numericValue)
                if (expanded != null) put("expanded", expanded)
                if (allowPrivateTextTransaction != null) {
                    put("allowPrivateTextTransaction", allowPrivateTextTransaction)
                }
                ChatGptWebConversationPath.canonicalProjectId(projectScopeId)?.let {
                    put("projectScopeId", it)
                }
                if (projectHints.isNotEmpty()) put("projectHints", JSONArray().apply {
                    projectHints.take(MAX_PROJECT_HINTS).forEach { project ->
                        put(JSONObject()
                            .put("id", project.id)
                            .put("title", project.title.take(MAX_PROJECT_TITLE_LENGTH))
                            .put("path", project.path)
                            .put("active", project.active))
                    }
                })
            }
            .toString()
        val encoded = JSONObject.quote(command)
        // onWebExecutionRequested() can resume a WebView that Android just paused. Post the
        // command to the next UI turn so Chromium has resumed before evaluating JavaScript.
        webView.post {
            if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
                return@post
            }
            webView.evaluateJavascript(
                "window.__elonChatGptBridge && window.__elonChatGptBridge.command($encoded);",
                null,
            )
        }
    }

    private fun isAllowedOrigin(origin: Uri): Boolean =
        origin.scheme == "https" && origin.host == "chatgpt.com" && origin.port == -1

    companion object {
        internal const val ADAPTER_VERSION = 243

        private val ADAPTER_ASSETS = listOf(
            "chatgpt_web_adapter_bootstrap.js",
            "chatgpt_web_adapter_authentication_policy.js",
            "chatgpt_web_private_conversation_directory.js",
            "chatgpt_web_adapter_conversation_directory_requests.js",
            "chatgpt_web_adapter_project_policy.js",
            "chatgpt_web_adapter_project_hints.js",
            "chatgpt_web_adapter_context_menu_policy.js",
            "chatgpt_web_adapter_control_labels.js",
            "chatgpt_web_adapter_project_choice_reveal.js",
            "chatgpt_web_adapter_conversation_history.js",
            "chatgpt_web_adapter_conversations.js",
            "chatgpt_web_adapter_message_action_policy.js",
            "chatgpt_web_adapter_message_portal_policy.js",
            "chatgpt_web_image_assets.js",
            "chatgpt_web_adapter_messages.js",
            "chatgpt_web_adapter_model_label_policy.js",
            "chatgpt_web_adapter_composer_option_policy.js",
            "chatgpt_web_adapter_composer_submenu.js",
            "chatgpt_web_adapter_composer_tool_state_policy.js",
            "chatgpt_web_adapter_composer_tool_selection.js",
            "chatgpt_web_adapter_action_target_policy.js",
            "chatgpt_web_adapter_attachment_policy.js",
            "chatgpt_web_dictation_runtime.js",
            "chatgpt_web_adapter_dictation_session_policy.js",
            "chatgpt_web_adapter_dictation_actions.js",
            "chatgpt_web_adapter_composer.js",
            "chatgpt_web_adapter_navigation_policy.js",
            "chatgpt_web_adapter_navigation.js",
            "chatgpt_web_adapter_page_semantic_policy.js",
            "chatgpt_web_adapter_temporary_chat.js",
            "chatgpt_web_adapter_form_controls.js",
            "chatgpt_web_adapter_control_ownership_policy.js",
            "chatgpt_web_adapter_overlay_policy.js",
            "chatgpt_web_adapter_form_commands.js",
            "chatgpt_web_adapter_disclosure_controls.js",
            "chatgpt_web_adapter_snapshot_scheduler.js",
            "chatgpt_web_adapter_streaming_policy.js",
            "chatgpt_web_stream_watchdog_probe.js",
            "chatgpt_web_stream_watchdog_acceptance.js",
            "chatgpt_web_adapter_skin.js",
            "chatgpt_web_adapter_realtime_voice_policy.js",
            "chatgpt_web_adapter_layout.js",
            "chatgpt_web_private_research_probe.js",
            "chatgpt_web_private_voice_relay.js",
            "chatgpt_web_realtime_data_channel_research.js",
            "chatgpt_web_realtime_voice_research.js",
            "chatgpt_web_private_transport_policy.js",
            "chatgpt_web_private_transport.js",
            "chatgpt_web_private_conversation_mutation.js",
            "chatgpt_web_private_read_aloud_transport.js",
            "chatgpt_web_private_read_aloud_adapter.js",
            "chatgpt_web_private_dictation_transport.js",
            "chatgpt_web_private_dictation_orchestrator.js",
            "chatgpt_web_private_text_transaction_policy.js",
            "chatgpt_web_private_text_transaction_relay.js",
            "chatgpt_web_private_stream_policy.js",
            "chatgpt_web_private_stream_transport.js",
            "chatgpt_web_private_send_observer.js",
            "chatgpt_web_text_transaction_orchestrator.js",
            "chatgpt_web_attachment_transport_observer.js",
            "chatgpt_web_adapter.js",
        )
        private const val BRIDGE_OBJECT = "elonChatGptNative"
        private const val ALLOWED_ORIGIN = "https://chatgpt.com"
        private const val PRIVATE_AUTH_CONTEXT_ASSET = "chatgpt_web_private_auth_context.js"
        private const val PRIVATE_FETCH_TAP_ASSET = "chatgpt_web_private_fetch_tap.js"
        private const val PRIVATE_SOCKET_TAP_ASSET = "chatgpt_web_private_socket_tap.js"
        private const val PRIVATE_TEXT_TRANSACTION_POLICY_ASSET =
            "chatgpt_web_private_text_transaction_policy.js"
        private const val PRIVATE_TEXT_TRANSACTION_RELAY_ASSET =
            "chatgpt_web_private_text_transaction_relay.js"
        private const val PRIVATE_REALTIME_VOICE_RESEARCH_ASSET =
            "chatgpt_web_realtime_voice_research.js"
        private const val PRIVATE_REALTIME_DATA_CHANNEL_RESEARCH_ASSET =
            "chatgpt_web_realtime_data_channel_research.js"
        private const val PRIVATE_REALTIME_VOICE_RELAY_ASSET =
            "chatgpt_web_private_voice_relay.js"
        private const val PRIVATE_CONVERSATION_DIRECTORY_ASSET =
            "chatgpt_web_private_conversation_directory.js"
        private const val ATTACHMENT_TRANSPORT_OBSERVER_ASSET =
            "chatgpt_web_attachment_transport_observer.js"
        private const val MAX_PROMPT_LENGTH = 20_000
        private const val MAX_CONVERSATION_PATH_LENGTH = 256
        private const val MAX_PROJECT_HINTS = 40
        private const val MAX_PROJECT_TITLE_LENGTH = 160
        private const val MAX_OPTION_ID_LENGTH = 64
        private const val MAX_UI_CONTROL_ID_LENGTH = 72
        private const val MAX_IMAGE_ASSET_HANDLE_LENGTH = 32
        private const val MAX_MESSAGE_CONTEXT_ID_LENGTH = 160
        private val REQUEST_ID = Regex("mcp_[a-z0-9]{1,32}")
    }
}

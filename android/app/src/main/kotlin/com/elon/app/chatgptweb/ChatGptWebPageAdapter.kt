package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.WebBridgeDocumentSession
import java.nio.charset.StandardCharsets
import org.json.JSONObject

internal class ChatGptWebPageAdapter(
    context: Context,
    private val webView: WebView,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onStateChanged: (State) -> Unit,
    private val onDocumentChanged: (WebBridgeDocumentSession.Snapshot) -> Unit = {},
) {
    enum class State {
        WEB_ONLY,
        CONNECTING,
        READY,
        UNSUPPORTED,
    }

    private val adapterScript = ADAPTER_ASSETS.joinToString("\n") { asset ->
        context.assets.open(asset).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }
    }
    private val mainHandler = Handler(Looper.getMainLooper())
    private val documentSession = WebBridgeDocumentSession()
    private val handshake = ChatGptWebBridgeHandshake(
        schedule = { delayMs, action -> mainHandler.postDelayed({ action() }, delayMs) },
        injectAndRequestSnapshot = ::injectAndRequestSnapshot,
    )
    private var listenerInstalled = false

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

    fun sendPrompt(prompt: String, expectedDraft: String, requestId: String? = null) = runCommand(
        action = "send_prompt",
        value = prompt.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
        requestId = requestId,
    )

    fun setDraft(value: String, expectedDraft: String, requestId: String) = runCommand(
        action = "set_draft",
        value = value.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
        requestId = requestId,
    )

    fun stopGeneration() = runCommand("stop_generation")

    fun stopGeneration(requestId: String) = runCommand("stop_generation", requestId = requestId)

    fun regenerateResponse() = runCommand("regenerate_response")

    fun regenerateResponse(requestId: String) = runCommand(
        "regenerate_response",
        requestId = requestId,
    )

    fun startNewConversation() = runCommand("new_conversation")

    fun startNewConversation(requestId: String) = runCommand("new_conversation", requestId = requestId)

    fun listConversations() = runCommand("list_conversations")

    fun listConversations(requestId: String) = runCommand("list_conversations", requestId = requestId)

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

    fun dismissComposerMenu() = runCommand("dismiss_composer_menu")

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

    fun startDictation() = runCommand("start_dictation")

    fun startDictation(requestId: String) = runCommand("start_dictation", requestId = requestId)

    fun cancelDictation() = runCommand("cancel_dictation")

    fun cancelDictation(requestId: String) = runCommand("cancel_dictation", requestId = requestId)

    fun submitDictation() = runCommand("submit_dictation")

    fun submitDictation(requestId: String) = runCommand("submit_dictation", requestId = requestId)

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

    fun requestSnapshot() = runCommand("snapshot")

    fun markReady() {
        if (documentSession.snapshot().adapterCurrent) onStateChanged(State.READY)
    }

    fun markLoginRequired() = onStateChanged(State.WEB_ONLY)

    fun dispose() {
        handshake.cancel()
        mainHandler.removeCallbacksAndMessages(null)
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
                requestSnapshot()
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
    ) {
        if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) return
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
            }
            .toString()
        val encoded = JSONObject.quote(command)
        webView.evaluateJavascript(
            "window.__elonChatGptBridge && window.__elonChatGptBridge.command($encoded);",
            null,
        )
    }

    private fun isAllowedOrigin(origin: Uri): Boolean =
        origin.scheme == "https" && origin.host == "chatgpt.com" && origin.port == -1

    companion object {
        internal const val ADAPTER_VERSION = 111

        private val ADAPTER_ASSETS = listOf(
            "chatgpt_web_adapter_bootstrap.js",
            "chatgpt_web_adapter_authentication_policy.js",
            "chatgpt_web_adapter_project_policy.js",
            "chatgpt_web_adapter_context_menu_policy.js",
            "chatgpt_web_adapter_conversation_history.js",
            "chatgpt_web_adapter_conversations.js",
            "chatgpt_web_adapter_message_action_policy.js",
            "chatgpt_web_adapter_messages.js",
            "chatgpt_web_adapter_model_label_policy.js",
            "chatgpt_web_adapter_composer_option_policy.js",
            "chatgpt_web_adapter_composer_tool_state_policy.js",
            "chatgpt_web_adapter_composer_tool_selection.js",
            "chatgpt_web_adapter_action_target_policy.js",
            "chatgpt_web_adapter_attachment_policy.js",
            "chatgpt_web_adapter_dictation_session_policy.js",
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
            "chatgpt_web_adapter_layout.js",
            "chatgpt_web_adapter.js",
        )
        private const val BRIDGE_OBJECT = "elonChatGptNative"
        private const val ALLOWED_ORIGIN = "https://chatgpt.com"
        private const val MAX_PROMPT_LENGTH = 20_000
        private const val MAX_CONVERSATION_PATH_LENGTH = 256
        private const val MAX_OPTION_ID_LENGTH = 64
        private const val MAX_UI_CONTROL_ID_LENGTH = 72
        private val REQUEST_ID = Regex("mcp_[a-z0-9]{1,32}")
    }
}

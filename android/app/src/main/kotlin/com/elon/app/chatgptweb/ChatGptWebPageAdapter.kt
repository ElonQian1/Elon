package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.nio.charset.StandardCharsets
import org.json.JSONObject

internal class ChatGptWebPageAdapter(
    context: Context,
    private val webView: WebView,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onStateChanged: (State) -> Unit,
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
            ChatGptWebProtocol.parse(payload, ADAPTER_VERSION)?.let { event ->
                if (event.completesHandshake()) handshake.acknowledge()
                onEvent(event)
            }
        }
        listenerInstalled = true
        onStateChanged(State.WEB_ONLY)
    }

    fun onPageReady(url: String) {
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) {
            onStateChanged(State.WEB_ONLY)
            return
        }
        if (!listenerInstalled) {
            onStateChanged(State.UNSUPPORTED)
            return
        }
        onStateChanged(State.CONNECTING)
        handshake.start()
    }

    fun onPageStarted(url: String) {
        handshake.cancel()
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
            handshake.start()
        }
    }

    fun sendPrompt(prompt: String, expectedDraft: String) = runCommand(
        action = "send_prompt",
        value = prompt.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
    )

    fun stopGeneration() = runCommand("stop_generation")

    fun regenerateResponse() = runCommand("regenerate_response")

    fun startNewConversation() = runCommand("new_conversation")

    fun listConversations() = runCommand("list_conversations")

    fun openConversation(path: String) = runCommand(
        action = "open_conversation",
        value = path.take(MAX_CONVERSATION_PATH_LENGTH),
    )

    fun startGoogleLogin() = runCommand("start_google_login")

    fun listModelOptions() = runCommand("list_model_options")

    fun listComposerTools() = runCommand("list_composer_tools")

    fun collectModelOptions() = runCommand("collect_model_options")

    fun collectComposerTools() = runCommand("collect_composer_tools")

    fun dismissComposerMenu() = runCommand("dismiss_composer_menu")

    fun selectModelOption(id: String) = runCommand("select_model_option", id.take(MAX_OPTION_ID_LENGTH))

    fun selectComposerTool(id: String) = runCommand("select_composer_tool", id.take(MAX_OPTION_ID_LENGTH))

    fun startDictation() = runCommand("start_dictation")

    fun removeAttachment(id: String) = runCommand("remove_attachment", id.take(MAX_OPTION_ID_LENGTH))

    fun listFeatures() = runCommand("list_navigation")

    fun collectFeatures() = runCommand("collect_navigation")

    fun selectFeature(id: String) = runCommand("select_navigation", id.take(MAX_OPTION_ID_LENGTH))

    fun dismissFeatures() = runCommand("dismiss_navigation")

    fun requestUiManifest() = runCommand("snapshot_ui_manifest")

    fun invokeUiControl(id: String) = runCommand("invoke_ui_control", id.take(MAX_UI_CONTROL_ID_LENGTH))

    fun requestSnapshot() = runCommand("snapshot")

    fun markReady() = onStateChanged(State.READY)

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
        webView.evaluateJavascript(adapterScript) {
            if (listenerInstalled && ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
                requestSnapshot()
            }
        }
    }

    private fun ChatGptWebEvent.completesHandshake(): Boolean = when (this) {
        is ChatGptWebEvent.Snapshot -> value.authenticated || value.composerReady
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
    ) {
        if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) return
        val command = JSONObject()
            .put("action", action)
            .apply {
                if (value != null) put("value", value)
                if (expectedDraft != null) put("expectedDraft", expectedDraft)
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
        internal const val ADAPTER_VERSION = 5

        private val ADAPTER_ASSETS = listOf(
            "chatgpt_web_adapter_bootstrap.js",
            "chatgpt_web_adapter_conversations.js",
            "chatgpt_web_adapter_messages.js",
            "chatgpt_web_adapter_composer.js",
            "chatgpt_web_adapter_navigation.js",
            "chatgpt_web_adapter_layout.js",
            "chatgpt_web_adapter.js",
        )
        private const val BRIDGE_OBJECT = "elonChatGptNative"
        private const val ALLOWED_ORIGIN = "https://chatgpt.com"
        private const val MAX_PROMPT_LENGTH = 20_000
        private const val MAX_CONVERSATION_PATH_LENGTH = 256
        private const val MAX_OPTION_ID_LENGTH = 64
        private const val MAX_UI_CONTROL_ID_LENGTH = 72
    }
}

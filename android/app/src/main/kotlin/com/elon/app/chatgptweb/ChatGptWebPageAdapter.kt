package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
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

    private val adapterScript = context.assets.open(ADAPTER_ASSET).use { input ->
        input.reader(StandardCharsets.UTF_8).readText()
    }
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
            ChatGptWebProtocol.parse(payload)?.let(onEvent)
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
        webView.evaluateJavascript(adapterScript, null)
    }

    fun onPageStarted(url: String) {
        val state = if (
            listenerInstalled && ChatGptWebNavigationPolicy.supportsEnhancedMode(url)
        ) {
            State.CONNECTING
        } else {
            State.WEB_ONLY
        }
        onStateChanged(state)
    }

    fun sendPrompt(prompt: String) = runCommand("send_prompt", prompt.take(MAX_PROMPT_LENGTH))

    fun stopGeneration() = runCommand("stop_generation")

    fun startNewConversation() = runCommand("new_conversation")

    fun startGoogleLogin() = runCommand("start_google_login")

    fun requestSnapshot() = runCommand("snapshot")

    fun markReady() = onStateChanged(State.READY)

    fun markLoginRequired() = onStateChanged(State.WEB_ONLY)

    fun dispose() {
        if (listenerInstalled && WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
            WebViewCompat.removeWebMessageListener(webView, BRIDGE_OBJECT)
        }
        listenerInstalled = false
    }

    private fun runCommand(action: String, value: String? = null) {
        if (!listenerInstalled || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) return
        val command = JSONObject()
            .put("action", action)
            .apply { if (value != null) put("value", value) }
            .toString()
        val encoded = JSONObject.quote(command)
        webView.evaluateJavascript(
            "window.__elonChatGptBridge && window.__elonChatGptBridge.command($encoded);",
            null,
        )
    }

    private fun isAllowedOrigin(origin: Uri): Boolean =
        origin.scheme == "https" && origin.host == "chatgpt.com" && origin.port == -1

    private companion object {
        const val ADAPTER_ASSET = "chatgpt_web_adapter.js"
        const val BRIDGE_OBJECT = "elonChatGptNative"
        const val ALLOWED_ORIGIN = "https://chatgpt.com"
        const val MAX_PROMPT_LENGTH = 20_000
    }
}

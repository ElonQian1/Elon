package com.elon.app.googleweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.WebBridgeConnectionState
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebBridgeReadinessPolicy
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebProtocol
import java.nio.charset.StandardCharsets
import org.json.JSONObject

internal class GoogleWebPageAdapter(
    context: Context,
    private val webView: WebView,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onStateChanged: (State) -> Unit,
) {
    enum class State { WEB_ONLY, CONNECTING, READY, UNSUPPORTED }

    private val script = ADAPTER_ASSETS.joinToString("\n") { asset ->
        context.assets.open(asset).use { input ->
            input.reader(StandardCharsets.UTF_8).readText()
        }
    }
    private val handler = Handler(Looper.getMainLooper())
    private val documentSession = WebBridgeDocumentSession()
    private var listenerInstalled = false

    fun install() {
        if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
            onStateChanged(State.UNSUPPORTED)
            return
        }
        WebViewCompat.addWebMessageListener(
            webView,
            BRIDGE_OBJECT,
            ALLOWED_ORIGINS,
        ) { _, message, origin, isMainFrame, _ ->
            if (!isMainFrame || !allowedOrigin(origin)) return@addWebMessageListener
            val raw = message.data ?: return@addWebMessageListener
            val root = runCatching { JSONObject(raw) }.getOrNull() ?: return@addWebMessageListener
            if (root.has("schema") && root.optString("providerId") != PROVIDER_ID) {
                return@addWebMessageListener
            }
            val parsed = ChatGptWebProtocol.parseMessage(raw, ADAPTER_VERSION)
                ?: return@addWebMessageListener
            val token = parsed.documentToken ?: return@addWebMessageListener
            val wasCurrent = documentSession.snapshot().adapterCurrent
            documentSession.accept(token) ?: return@addWebMessageListener
            if (!wasCurrent) onStateChanged(State.READY)
            onEvent(parsed.event)
        }
        listenerInstalled = true
        onStateChanged(State.WEB_ONLY)
    }

    fun onPageStarted(url: String) {
        handler.removeCallbacksAndMessages(null)
        documentSession.beginPage()
        onStateChanged(if (supports(url) && listenerInstalled) State.CONNECTING else State.WEB_ONLY)
    }

    fun onPageReady(url: String) {
        val pageSupported = supports(url)
        if (pageSupported && listenerInstalled && documentSession.snapshot().pageGeneration == 0L) {
            documentSession.ensurePage()
        }
        onStateChanged(when (WebBridgeReadinessPolicy.stateAfterPageReady(
            listenerInstalled = listenerInstalled,
            pageSupported = pageSupported,
            document = documentSession.snapshot(),
        )) {
            WebBridgeConnectionState.WEB_ONLY -> State.WEB_ONLY
            WebBridgeConnectionState.CONNECTING -> State.CONNECTING
            WebBridgeConnectionState.READY -> State.READY
            WebBridgeConnectionState.UNSUPPORTED -> State.UNSUPPORTED
        })
        if (!pageSupported || !listenerInstalled) return
        RETRY_DELAYS_MS.forEach { delay -> handler.postDelayed(::injectAndSnapshot, delay) }
    }

    fun onHostResumed(url: String?) {
        if (supports(url) && listenerInstalled) {
            documentSession.ensurePage()
            injectAndSnapshot()
        }
    }

    fun requestSnapshot() = runCommand("snapshot")

    fun sendPrompt(value: String, expectedDraft: String) = runCommand(
        action = "send_prompt",
        value = value.take(MAX_PROMPT_LENGTH),
        expectedDraft = expectedDraft.take(MAX_PROMPT_LENGTH),
    )

    fun stopGeneration() = runCommand("stop_generation")

    fun startNewConversation() = runCommand("new_conversation")

    fun dispose() = handler.removeCallbacksAndMessages(null)

    private fun injectAndSnapshot() {
        if (!supports(webView.url)) return
        val document = documentSession.ensurePage()
        val bootstrap = "window.__elonGoogleWebAdapterVersion=$ADAPTER_VERSION;" +
            "window.__elonGoogleWebDocumentToken=${JSONObject.quote(document.documentToken)};\n$script"
        webView.evaluateJavascript(bootstrap) { runCommand("snapshot") }
    }

    private fun runCommand(
        action: String,
        value: String = "",
        expectedDraft: String = "",
    ) {
        val command = JSONObject()
            .put("action", action)
            .put("value", value)
            .put("expectedDraft", expectedDraft)
            .toString()
        val encoded = JSONObject.quote(command)
        webView.evaluateJavascript(
            "window.__elonGoogleWebBridge&&window.__elonGoogleWebBridge.command($encoded);",
            null,
        )
    }

    private fun supports(url: String?): Boolean = GoogleWebNavigationPolicy.supportsAiMode(url)

    private fun allowedOrigin(origin: Uri): Boolean =
        origin.scheme == "https" && origin.port == -1 && origin.host?.lowercase() in ALLOWED_HOSTS

    companion object {
        const val ADAPTER_VERSION = 2
        private val ADAPTER_ASSETS = listOf(
            "google_web_message_extractor.js",
            "google_web_adapter.js",
        )
        private const val BRIDGE_OBJECT = "elonGoogleWebNative"
        private const val PROVIDER_ID = "google_web"
        private const val MAX_PROMPT_LENGTH = 20_000
        private val ALLOWED_HOSTS = setOf("google.com", "www.google.com")
        private val ALLOWED_ORIGINS = setOf("https://google.com", "https://www.google.com")
        private val RETRY_DELAYS_MS = longArrayOf(0L, 600L, 1_800L, 4_000L)
    }
}

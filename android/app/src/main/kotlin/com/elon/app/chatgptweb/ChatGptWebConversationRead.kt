package com.elon.app.chatgptweb

import android.content.Context
import android.webkit.WebView
import com.elon.app.WebBridgeDocumentSession
import org.json.JSONObject

/** Explicit read results stay here, outside the UI snapshot and diagnostic timeline. */
internal class ChatGptWebConversationRead(
    context: Context,
    private val webView: WebView,
    private val document: () -> WebBridgeDocumentSession.Snapshot,
) {
    private val script by lazy {
        listOf("chatgpt_web_conversation_projection.js", "chatgpt_web_conversation_reader.js")
            .joinToString("\n") { context.assets.open(it).bufferedReader().use { reader -> reader.readText() } }
    }
    private var signature = ""
    private var owner = ""
    private var reading = false
    private var result: JSONObject? = null
    private var receivedAt = 0L

    fun read(args: JSONObject): JSONObject {
        val binding = document()
        if (!binding.adapterCurrent || !ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
            clear()
            return failed("reader_unavailable")
        }
        val request = JSONObject().put("conversation_id", args.optString("conversation_id"))
            .put("request_id", args.optString("request_id"))
            .put("cursor", args.optString("message_cursor"))
        val key = request.toString()
        if (key != signature || owner != binding.documentToken) {
            if (reading && owner == binding.documentToken) return failed("reader_busy")
            clear()
            signature = key
            owner = binding.documentToken
        }
        val previous = result?.takeIf { android.os.SystemClock.elapsedRealtime() - receivedAt < 2000 }
        // Re-evaluate even completed requests: the page checks the live account before replay.
        if (!reading) {
            reading = true
            val token = binding.documentToken
            val code = "(function(){if(location.origin!=='https://chatgpt.com'||" +
                "window.__elonChatGptDocumentToken!==${JSONObject.quote(token)})return null;\n" +
                "$script\nreturn window.__elonConversationReader.run($request);})()"
            webView.evaluateJavascript(code) { raw ->
                if (signature == key && owner == token && document().documentToken == token) {
                    reading = false
                    result = if (raw.length <= 60 * 1024) runCatching { JSONObject(raw) }.getOrNull()
                        ?: failed("reader_context_changed") else failed("source_limit")
                    receivedAt = android.os.SystemClock.elapsedRealtime()
                }
            }
        }
        // Consume once, so cached content is never replayed before a fresh identity check.
        result = null
        return (previous ?: JSONObject().put("status", "pending"))
            .put("request_id", request.optString("request_id"))
            .put("source", "apk")
    }

    private fun clear() { signature = ""; owner = ""; reading = false; result = null }
    private fun failed(code: String) = JSONObject().put("status", "failed").put("error", code)
}

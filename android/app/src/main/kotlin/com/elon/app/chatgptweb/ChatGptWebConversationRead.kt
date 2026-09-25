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
    private val requestExecution: () -> Unit,
) {
    private val script by lazy {
        listOf("chatgpt_web_private_json_request.js", "chatgpt_web_private_auth_context.js",
            "chatgpt_web_text_blocks.js", "chatgpt_web_private_file_citation.js",
            "chatgpt_web_private_history_projection.js", "chatgpt_web_private_image_pointer.js",
            "chatgpt_web_private_content_source.js", "chatgpt_web_private_library_download.js",
            "chatgpt_web_private_library_raster_policy.js", "chatgpt_web_private_canvas_text_export.js",
            "chatgpt_web_private_generated_image_download.js", "chatgpt_web_private_file_download.js",
            "chatgpt_web_conversation_content.js", "chatgpt_web_conversation_projection.js",
            "chatgpt_web_conversation_assets.js", "chatgpt_web_conversation_reader.js")
            .joinToString("\n") { context.assets.open(it).bufferedReader().use { reader -> reader.readText() } }
    }
    private var signature = ""
    private var owner = ""
    private var reading = false
    private var result: JSONObject? = null
    private var receivedAt = 0L

    fun read(args: JSONObject): JSONObject {
        val binding = document()
        val expectedUrl = webView.url
        val rejection = ChatGptWebConversationReadAdmission.rejection(expectedUrl, binding)
        if (rejection != null) {
            clear()
            return failed(rejection).put("request_id", args.optString("request_id"))
        }
        // Every authorized poll renews the existing bounded execution lease.
        // Direct evaluation alone can run while WebView network/timers are paused.
        requestExecution()
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
                "(window.__elonChatGptDocumentToken&&window.__elonChatGptDocumentToken!==${JSONObject.quote(token)}))return null;\n" +
                "window.__elonChatGptPrivateAuthContextEnabled=true;\n" +
                "$script\nreturn window.__elonConversationReader.run($request);})()"
            webView.evaluateJavascript(code) { raw ->
                if (signature == key && owner == token && document().documentToken == token) {
                    if (webView.url != expectedUrl) {
                        clear()
                        return@evaluateJavascript
                    }
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

package com.elon.app.chatgptweb

import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.view.ViewGroup
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.beginWebChatBackgroundInteraction
import java.util.UUID

/** A separate document, sharing identity but never personal conversation/navigation state. */
internal class GroupWebAiExecutor(
    private val activity: AppCompatActivity,
    private val prompt: String,
    private val authorize: ((Boolean) -> Unit) -> Unit,
    private val onResult: (String) -> Unit,
    private val onFailure: (Boolean) -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private var webView: WebView? = null
    private var adapter: ChatGptWebPageAdapter? = null
    private var finished = false
    private var dispatching = false
    private var dispatched = false
    private val commandId = UUID.randomUUID().toString()
    private val timeout = Runnable { fail() }

    fun start() {
        handler.postDelayed(timeout, 60_000L)
        val view = createChatGptBackgroundWebView(activity, null, {}, { it.onReceiveValue(null) })
        webView = view
        activity.findViewById<ViewGroup>(android.R.id.content).addView(view, 0,
            ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        view.beginWebChatBackgroundInteraction()
        view.settings.mediaPlaybackRequiresUserGesture = true
        val bridge = ChatGptWebPageAdapter(activity, view, ::event, {})
        adapter = bridge
        view.webViewClient = ChatGptWebViewClient(
            onPageStarted = { bridge.onPageStarted(it) },
            onPageReady = { bridge.onPageReady(it) },
            onBlockedNavigation = { fail() },
            onPageError = { fail() },
            rewriteAllowedMainFrameUrl = { null },
        )
        bridge.install()
        ChatGptWebProxyController(activity).prepare {
            if (!finished) view.loadUrl("https://chatgpt.com/?temporary-chat=true")
        }
    }

    private fun event(event: ChatGptWebEvent) {
        if (finished) return
        when (event) {
            is ChatGptWebEvent.Snapshot -> snapshot(event.value)
            is ChatGptWebEvent.CommandResult ->
                if (event.requestId == commandId && !event.ok) fail()
            else -> Unit
        }
    }

    private fun snapshot(value: ChatGptWebSnapshot) {
        if (!dispatched) {
            if (value.loginRequired) { fail(); return }
            if (dispatching || (!value.composerReady && !value.privateSendReady) || value.streaming) return
            val uri = Uri.parse(value.url)
            if (uri.host != "chatgpt.com" || uri.path !in listOf("", "/") ||
                uri.getQueryParameter("temporary-chat") != "true" ||
                value.messages.isNotEmpty() || value.draft.isNotBlank()
            ) return
            dispatching = true
            // Once authorization is attempted, a lost response must never trigger paid fallback.
            authorize { permitted ->
                if (finished) return@authorize
                if (!permitted) { fail(); return@authorize }
                dispatched = true
                handler.removeCallbacks(timeout)
                handler.postDelayed(timeout, 180_000L)
                adapter?.sendPrompt(prompt, "", commandId, allowPrivateTextTransaction = true)
            }
            return
        }
        val reply = completedReply(value.messages, prompt, value.streaming, value.privateStreamState) ?: return
        finish()
        onResult(reply)
    }

    fun cancel() = fail()

    private fun fail() {
        if (finished) return
        val uncertain = dispatching || dispatched
        finish()
        onFailure(uncertain)
    }

    private fun finish() {
        finished = true
        handler.removeCallbacks(timeout)
        adapter?.dispose()
        adapter = null
        webView?.stopLoading()
        (webView?.parent as? ViewGroup)?.removeView(webView)
        webView?.destroy()
        webView = null
    }

    companion object {
        fun completedReply(messages: List<ChatGptWebMessage>, prompt: String, streaming: Boolean, streamState: String): String? {
            val expected = normalizePrompt(prompt)
            val source = messages.indexOfLast { it.role == "user" && normalizePrompt(it.content) == expected }
            if (source < 0 || streaming) return null
            val reply = messages.drop(source + 1).lastOrNull {
                it.role == "assistant" && it.content.isNotBlank()
            } ?: return null
            if (streamState != "completed" && reply.state !in listOf("complete", "completed", "finished_successfully")) return null
            return reply.content
        }
        private fun normalizePrompt(value: String) = value.trim().replace(Regex("\\s+"), " ")
    }
}

package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.webkit.CookieManager
import android.view.ViewGroup
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.beginWebChatBackgroundInteraction
import com.elon.app.WebChatProviderId
import com.elon.app.googleweb.GoogleWebPageAdapter
import com.elon.app.googleweb.GoogleWebViewClient
import com.elon.app.googleweb.GoogleWebResponseRefreshCoordinator

/** One isolated document; shared identity, no personal history stores or navigation. */
internal class GroupWebAiSession(
    private val activity: AppCompatActivity,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onFailure: () -> Unit,
    private val provider: WebChatProviderId = WebChatProviderId.CHATGPT_WEB,
    private val observe: (String) -> Unit = {},
) {
    private var closed = false
    private val handler = Handler(Looper.getMainLooper())
    private val view = createChatGptBackgroundWebView(activity, null, {}, { it.onReceiveValue(null) })
    private val chatGpt = if (provider == WebChatProviderId.CHATGPT_WEB)
        ChatGptWebPageAdapter(activity, view, ::event, { observe("bridge_${it.name.lowercase()}") }) else null
    private val google = if (provider == WebChatProviderId.GOOGLE_WEB)
        GoogleWebPageAdapter(activity, view, ::event, {}) else null
    val adapter: ChatGptWebPageAdapter get() = requireNotNull(chatGpt)
    val documentUrl: String get() = view.url.orEmpty()
    fun isReady(snapshot: ChatGptWebSnapshot): Boolean =
        !closed && GroupWebAiSessionPolicy.ready(snapshot, provider, documentUrl)
    private val refresh = GoogleWebResponseRefreshCoordinator(
        requestSnapshot = { google?.requestSnapshot() },
        schedule = { task, delay -> handler.postDelayed(task, delay) }, cancel = handler::removeCallbacks,
    )

    private fun event(event: ChatGptWebEvent) {
        if (closed) return
        if (event is ChatGptWebEvent.CommandResult && event.action == "send_prompt" && event.ok) refresh.onSendConfirmed()
        if (event is ChatGptWebEvent.Snapshot) {
            val snapshot = event.value
            val user = snapshot.messages.indexOfLast { it.role == "user" }
            refresh.onSnapshot(snapshot.messages.getOrNull(user)?.content,
                user >= 0 && snapshot.messages.drop(user + 1).any { it.role == "assistant" }, snapshot.streaming)
        }
        onEvent(event)
    }

    fun sendPrompt(prompt: String, requestId: String) {
        if (closed) return
        if (google != null) {
            refresh.onSendStarted(prompt)
            google.sendPrompt(prompt, "", requestId)
        } else adapter.sendPrompt(prompt, "", requestId, allowPrivateTextTransaction = true)
    }

    fun start() {
        activity.findViewById<ViewGroup>(android.R.id.content).addView(view, 0,
            ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        view.beginWebChatBackgroundInteraction()
        view.settings.mediaPlaybackRequiresUserGesture = true
        view.settings.allowContentAccess = false
        CookieManager.getInstance().apply { setAcceptCookie(true); setAcceptThirdPartyCookies(view, true) }
        view.webViewClient = if (google != null) GoogleWebViewClient(
            onPageStarted = { google.onPageStarted(it) }, onPageReady = { google.onPageReady(it) },
            onBlockedNavigation = { if (!closed) onFailure() }, onPageError = { if (!closed) onFailure() },
        ) else ChatGptWebViewClient(
            onPageStarted = { observe("page_started"); adapter.onPageStarted(it) },
            onPageReady = { observe("page_loaded"); adapter.onPageReady(it) },
            onBlockedNavigation = { if (!closed) { observe("navigation_blocked"); onFailure() } },
            onPageError = { if (!closed) { observe("page_error"); onFailure() } },
            rewriteAllowedMainFrameUrl = { null },
        )
        if (google != null) google.install() else adapter.install()
        ChatGptWebProxyController(activity).prepare {
            if (!closed) view.loadUrl(GroupWebAiSessionPolicy.startUrl(provider))
        }
    }

    fun close() {
        if (closed) return
        closed = true
        refresh.stop()
        handler.removeCallbacksAndMessages(null)
        google?.dispose()
        chatGpt?.dispose()
        view.stopLoading()
        (view.parent as? ViewGroup)?.removeView(view)
        view.destroy()
    }

}

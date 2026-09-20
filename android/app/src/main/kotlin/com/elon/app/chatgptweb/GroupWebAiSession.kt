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
    private val onFailure: (GroupWebAiFailureReason) -> Unit,
    private val provider: WebChatProviderId = WebChatProviderId.CHATGPT_WEB,
    private val observe: (String) -> Unit = {},
) {
    private var closed = false
    private val handler = Handler(Looper.getMainLooper())
    private val view = createChatGptBackgroundWebView(activity, null, {}, { it.onReceiveValue(null) })
    private val chatGpt = if (provider == WebChatProviderId.CHATGPT_WEB)
        ChatGptWebPageAdapter(activity, view, ::event, { observe("bridge_${it.name.lowercase()}") },
            allowTemporaryTextDispatch = true) else null
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
    private val touchDispatcher = ChatGptWebTouchDispatcher(view)
    private val modelTouch = ChatGptWebTouchRequestHandler(
        webView = { view.takeUnless { closed } },
        pageAdapter = { chatGpt.takeUnless { closed } },
        touchDispatcher = { touchDispatcher.takeUnless { closed } },
        isInteractiveSurface = { true },
        runBackgroundInteraction = { _, _ -> false },
        interactionRequested = { observe("model_touch_requested") },
        dismissComposerOptions = {},
        scheduleModelOptions = {
            handler.postDelayed({
                if (!closed) { chatGpt?.collectModelOptions(); chatGpt?.requestUiManifest() }
            }, ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS)
        },
        scheduleToolOptions = {},
        onDispatchFailed = {
            if (!closed) { observe("model_touch_failed"); onFailure(GroupWebAiFailureReason.MODEL) }
        },
    )

    private fun event(event: ChatGptWebEvent) {
        if (closed) return
        if (event is ChatGptWebEvent.WebTouchRequest) {
            if (provider == WebChatProviderId.CHATGPT_WEB && GroupWebAiSessionPolicy.allowsModelTouch(event.purpose)) {
                modelTouch.handle(event)
            }
            return
        }
        if (event is ChatGptWebEvent.ComposerControls && event.section == "model") {
            observe(if (event.options.isEmpty()) "model_catalog_empty" else "model_catalog")
            // Read slider controls after the menu exists, not only before opening it.
            chatGpt?.requestUiManifest()
        }
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
            onBlockedNavigation = { pageFailure("navigation_blocked") }, onPageError = { pageFailure("page_error") },
        ) else ChatGptWebViewClient(
            onPageStarted = { observe("page_started"); adapter.onPageStarted(it) },
            onPageReady = { observe("page_loaded"); adapter.onPageReady(it) },
            onBlockedNavigation = { pageFailure("navigation_blocked") },
            onPageError = { pageFailure("page_error") },
            rewriteAllowedMainFrameUrl = { null },
        )
        if (google != null) google.install() else adapter.install()
        ChatGptWebProxyController(activity).prepare {
            if (!closed) view.loadUrl(GroupWebAiSessionPolicy.startUrl(provider))
        }
    }

    private fun pageFailure(stage: String) {
        if (closed) return
        observe(stage)
        onFailure(GroupWebAiFailureReason.PAGE)
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

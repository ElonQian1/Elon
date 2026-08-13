package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.graphics.Color
import android.view.View
import android.webkit.CookieManager
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity

internal class ChatGptBackgroundSession(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val onSnapshot: (ChatGptWebSnapshot) -> Unit,
    private val onStateChanged: (State, String?) -> Unit,
    private val onComposerOptions: (List<ChatGptWebComposerOption>) -> Unit,
    private val onCommandResult: (ChatGptWebEvent.CommandResult) -> Unit,
) {
    enum class State(val wireValue: String) {
        IDLE("idle"),
        LOADING("loading"),
        READY("ready"),
        LOGIN_REQUIRED("login_required"),
        ERROR("error"),
    }

    private val cookieManager = CookieManager.getInstance()
    private val sessionRestorer = ChatGptWebSessionRestorer(activity)
    private val sessionContinuity = ChatGptWebSessionContinuity()
    private val proxyController = ChatGptWebProxyController(activity)
    private var webView: WebView? = null
    private var pageAdapter: ChatGptWebPageAdapter? = null
    private var touchDispatcher: ChatGptWebTouchDispatcher? = null
    private var latestSnapshot: ChatGptWebSnapshot? = null
    private var state = State.IDLE

    fun activate() {
        ensureInitialized()
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
        latestSnapshot?.let(onSnapshot)
    }

    fun onHostResumed() {
        if (webView == null) return
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
    }

    fun onHostPaused() {
        if (webView == null) return
        cookieManager.flush()
        webView?.onPause()
    }

    fun currentSnapshot(): ChatGptWebSnapshot? = latestSnapshot

    fun state(): State = state

    fun canSend(): Boolean = state == State.READY && latestSnapshot?.composerReady == true

    fun sendPrompt(prompt: String): Boolean {
        val adapter = pageAdapter ?: return false
        val snapshot = latestSnapshot ?: return false
        if (!canSend()) return false
        adapter.sendPrompt(prompt, snapshot.draft)
        return true
    }

    fun requestModelOptions(): Boolean {
        if (state != State.READY) return false
        pageAdapter?.listModelOptions()
        return true
    }

    fun selectModel(id: String) {
        pageAdapter?.selectModelOption(id)
    }

    fun stopGeneration() {
        pageAdapter?.stopGeneration()
    }

    fun startNewConversation() {
        pageAdapter?.startNewConversation()
    }

    fun destroy() {
        pageAdapter?.dispose()
        pageAdapter = null
        touchDispatcher = null
        webView?.apply {
            stopLoading()
            webChromeClient = null
            host.removeView(this)
            destroy()
        }
        webView = null
        latestSnapshot = null
        updateState(State.IDLE)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun ensureInitialized() {
        if (webView != null) return
        WebView.setWebContentsDebuggingEnabled(false)
        val view = WebView(activity).apply {
            setBackgroundColor(Color.TRANSPARENT)
            alpha = 0.01f
            isClickable = false
            isFocusable = false
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                allowFileAccess = false
                allowContentAccess = true
                javaScriptCanOpenWindowsAutomatically = false
                setSupportMultipleWindows(false)
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
                builtInZoomControls = false
                displayZoomControls = false
            }
            webChromeClient = WebChromeClient()
        }
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(view, true)
        val adapter = ChatGptWebPageAdapter(
            context = activity,
            webView = view,
            onEvent = ::handleEvent,
            onStateChanged = ::handleAdapterState,
        )
        view.webViewClient = ChatGptWebViewClient(
            onPageStarted = { url ->
                adapter.onPageStarted(url)
                updateState(State.LOADING)
            },
            onPageReady = { url ->
                cookieManager.flush()
                sessionRestorer.onPageReady(url)
                adapter.onPageReady(url)
            },
            onBlockedNavigation = { hostName ->
                updateState(State.ERROR, "官网导航被拦截：$hostName")
            },
            onPageError = { detail -> updateState(State.ERROR, detail) },
            rewriteAllowedMainFrameUrl = { null },
        )
        host.addView(
            view,
            0,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT,
            ),
        )
        webView = view
        pageAdapter = adapter
        touchDispatcher = ChatGptWebTouchDispatcher(view)
        adapter.install()
        proxyController.prepare { status ->
            if (activity.isFinishing || activity.isDestroyed || webView !== view) return@prepare
            status.error?.let {
                updateState(State.ERROR, it)
                return@prepare
            }
            updateState(State.LOADING)
            view.loadUrl(chatRestorableUrl(sessionRestorer.restoreUrl()))
        }
    }

    private fun handleEvent(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.Snapshot -> {
                val snapshot = sessionContinuity.reconcile(event.value)
                latestSnapshot = snapshot
                when {
                    snapshot.loginRequired || snapshot.pageKind == "auth" -> {
                        pageAdapter?.markLoginRequired()
                        updateState(State.LOGIN_REQUIRED)
                    }
                    snapshot.authenticated && snapshot.composerReady -> {
                        pageAdapter?.markReady()
                        updateState(State.READY)
                    }
                    else -> updateState(State.LOADING)
                }
                onSnapshot(snapshot)
            }
            is ChatGptWebEvent.ComposerControls -> {
                if (event.section == "model") onComposerOptions(event.options)
            }
            is ChatGptWebEvent.CommandResult -> {
                onCommandResult(event)
                if (event.ok) {
                    pageAdapter?.requestSnapshot()
                } else {
                    onStateChanged(state, event.detail.ifBlank { "官网操作失败" })
                }
            }
            is ChatGptWebEvent.AdapterReady,
            is ChatGptWebEvent.ConversationList,
            is ChatGptWebEvent.FeatureNavigation,
            is ChatGptWebEvent.UiManifest -> Unit
            is ChatGptWebEvent.WebTouchRequest -> handleWebTouchRequest(event)
        }
    }

    private fun chatRestorableUrl(savedUrl: String): String {
        val path = runCatching { java.net.URI(savedUrl).path.orEmpty() }.getOrDefault("")
        return if (path == "/" || path.startsWith("/c/") || path.startsWith("/g/")) savedUrl
        else ChatGptWebNavigationPolicy.START_URL
    }

    private fun handleWebTouchRequest(event: ChatGptWebEvent.WebTouchRequest) {
        val view = webView ?: return
        val adapter = pageAdapter ?: return
        touchDispatcher?.dispatch(event) { dispatched ->
            if (!dispatched) {
                onStateChanged(state, "官网控件操作未就绪")
                return@dispatch
            }
            when (event.purpose) {
                "list_model_options", "open_model_submenu" -> view.postDelayed(
                    adapter::collectModelOptions,
                    ChatGptWebTestActivity.COMPOSER_MENU_SETTLE_MS,
                )
                "list_composer_tools", "open_composer_tools_submenu" -> view.postDelayed(
                    adapter::collectComposerTools,
                    ChatGptWebTestActivity.COMPOSER_MENU_SETTLE_MS,
                )
                "list_navigation" -> view.postDelayed(
                    adapter::collectFeatures,
                    ChatGptWebTestActivity.NAVIGATION_SETTLE_MS,
                )
                "select_model_option", "select_composer_tool", "remove_attachment",
                "start_dictation", "cancel_dictation", "submit_dictation" -> view.postDelayed(
                    adapter::requestSnapshot,
                    ChatGptWebTestActivity.COMPOSER_MENU_SETTLE_MS,
                )
                "select_navigation", "invoke_ui_control", "regenerate_open_menu", "regenerate_retry" ->
                    view.postDelayed(adapter::requestSnapshot, ChatGptWebTestActivity.NAVIGATION_SETTLE_MS)
            }
        }
    }

    private fun handleAdapterState(adapterState: ChatGptWebPageAdapter.State) {
        if (adapterState == ChatGptWebPageAdapter.State.UNSUPPORTED) {
            updateState(State.ERROR, "当前 WebView 不支持网页 AI 语义桥接")
        }
    }

    private fun updateState(next: State, detail: String? = null) {
        state = next
        onStateChanged(next, detail)
    }
}

package com.elon.app.googleweb

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.View
import android.webkit.CookieManager
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebChromeClient
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebProxyController
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.WebChatSnapshotStore
import java.time.LocalDate

internal class GoogleWebBackgroundSession(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val onSnapshot: (ChatGptWebSnapshot) -> Unit,
    private val onStateChanged: (State, String?) -> Unit,
    private val onCommandResult: (ChatGptWebEvent.CommandResult) -> Unit,
    private val onConversationIndexChanged: (ChatGptWebConversationIndexState) -> Unit,
) {
    enum class State(val wireValue: String) {
        IDLE("idle"), LOADING("loading"), READY("ready"), ERROR("error")
    }

    private val cookieManager = CookieManager.getInstance()
    private val proxyController = ChatGptWebProxyController(activity)
    private val conversationStore = GoogleWebConversationStore(activity)
    private val snapshotStore = WebChatSnapshotStore(activity, "google")
    private val preferences = activity.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
    private val handler = Handler(Looper.getMainLooper())
    private var webView: WebView? = null
    private var pageAdapter: GoogleWebPageAdapter? = null
    private var latestSnapshot: ChatGptWebSnapshot? = snapshotStore.restore()
    private var activePath: String? = null
    private var state = State.IDLE

    fun activate() {
        latestSnapshot?.let(onSnapshot)
        onConversationIndexChanged(conversationIndex())
        ensureInitialized()
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
    }

    fun deactivate() = Unit

    fun onHostResumed() {
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
    }

    fun onHostPaused() {
        cookieManager.flush()
        webView?.onPause()
    }

    fun currentSnapshot(): ChatGptWebSnapshot? = latestSnapshot

    fun state(): State = state

    fun canSend(): Boolean = state == State.READY && latestSnapshot?.composerReady == true

    fun sendPrompt(prompt: String): Boolean {
        val snapshot = latestSnapshot ?: return false
        if (!canSend()) return false
        pageAdapter?.sendPrompt(prompt, snapshot.draft) ?: return false
        return true
    }

    fun stopGeneration() = pageAdapter?.stopGeneration()

    fun startNewConversation() {
        activePath = null
        pageAdapter?.startNewConversation()
    }

    fun currentConversationPath(): String? = activePath

    fun conversationIndex(): ChatGptWebConversationIndexState = conversationStore.index(activePath)

    fun requestConversationIndex(): Boolean {
        onConversationIndexChanged(conversationIndex())
        pageAdapter?.requestSnapshot()
        return webView != null
    }

    fun openConversation(path: String): Boolean {
        val url = conversationStore.restorableUrl(path) ?: return false
        if (!GoogleWebNavigationPolicy.supportsAiMode(url)) return false
        activePath = path
        updateState(State.LOADING)
        webView?.loadUrl(url) ?: return false
        return true
    }

    fun openProject(path: String): Boolean = false

    fun destroy() {
        handler.removeCallbacksAndMessages(null)
        pageAdapter?.dispose()
        pageAdapter = null
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
                allowContentAccess = false
                javaScriptCanOpenWindowsAutomatically = false
                setSupportMultipleWindows(false)
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
            }
            webChromeClient = WebChromeClient()
        }
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(view, true)
        val adapter = GoogleWebPageAdapter(
            context = activity,
            webView = view,
            onEvent = ::handleEvent,
            onStateChanged = ::handleAdapterState,
        )
        view.webViewClient = GoogleWebViewClient(
            onPageStarted = { url ->
                adapter.onPageStarted(url)
                updateState(State.LOADING)
            },
            onPageReady = { url ->
                cookieManager.flush()
                adapter.onPageReady(url)
            },
            onBlockedNavigation = { hostName ->
                updateState(State.ERROR, "Google 官方导航被拦截：$hostName")
            },
            onPageError = { detail -> updateState(State.ERROR, detail) },
        )
        host.addView(view, 0, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT,
        ))
        webView = view
        pageAdapter = adapter
        adapter.install()
        proxyController.prepare { status ->
            if (activity.isFinishing || activity.isDestroyed || webView !== view) return@prepare
            status.error?.let { updateState(State.ERROR, it); return@prepare }
            val restored = GoogleWebNavigationPolicy.sanitizeRestorableUrl(
                preferences.getString(KEY_LAST_URL, null),
            ) ?: GoogleWebNavigationPolicy.START_URL
            updateState(State.LOADING)
            view.loadUrl(restored)
        }
    }

    private fun handleEvent(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.Snapshot -> {
                latestSnapshot = event.value
                if (event.value.composerReady && !event.value.streaming) snapshotStore.save(event.value)
                val pageTitle = event.value.title.trim().takeUnless {
                    it.equals("Google", ignoreCase = true) ||
                        it.equals("Google AI 模式", ignoreCase = true)
                }
                val title = pageTitle
                    ?: event.value.messages.firstOrNull { it.role == "user" }?.content.orEmpty()
                activePath = conversationStore.observe(event.value.url, title, LocalDate.now())
                    ?: conversationStore.currentPath(event.value.url)
                GoogleWebNavigationPolicy.sanitizeRestorableUrl(event.value.url)?.let { url ->
                    preferences.edit().putString(KEY_LAST_URL, url).apply()
                }
                updateState(if (event.value.composerReady) State.READY else State.LOADING)
                onConversationIndexChanged(conversationIndex())
                onSnapshot(event.value)
            }
            is ChatGptWebEvent.CommandResult -> {
                if (event.action == DOM_DIAGNOSTICS_ACTION) {
                    Log.i(DOM_DIAGNOSTICS_TAG, event.detail.take(160))
                    return
                }
                onCommandResult(event)
                if (event.ok || event.action == "send_prompt") {
                    handler.postDelayed({ pageAdapter?.requestSnapshot() }, 500L)
                }
            }
            is ChatGptWebEvent.AdapterReady -> Unit
            else -> Unit
        }
    }

    private fun handleAdapterState(next: GoogleWebPageAdapter.State) {
        when (next) {
            GoogleWebPageAdapter.State.READY -> Unit
            GoogleWebPageAdapter.State.UNSUPPORTED -> updateState(
                State.ERROR,
                "当前 WebView 不支持 Google 网页 AI 语义桥接",
            )
            GoogleWebPageAdapter.State.CONNECTING -> updateState(State.LOADING)
            GoogleWebPageAdapter.State.WEB_ONLY -> Unit
        }
    }

    private fun updateState(next: State, detail: String? = null) {
        state = next
        onStateChanged(next, detail)
    }

    private companion object {
        const val PREFERENCES_NAME = "google_web_session"
        const val KEY_LAST_URL = "last_ai_mode_url"
        const val DOM_DIAGNOSTICS_ACTION = "dom_diagnostics"
        const val DOM_DIAGNOSTICS_TAG = "ElonGoogleWebDom"
    }
}

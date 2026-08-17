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
import com.elon.app.WebChatSessionRecoveryCoordinator
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebProxyController
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.WebChatSendContextPolicy
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
    private val conversationSnapshotStore = GoogleWebConversationSnapshotStore(activity)
    private val conversationNavigation = GoogleWebConversationNavigationCoordinator()
    private val snapshotStore = WebChatSnapshotStore(activity, "google")
    private val preferences = activity.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
    private val handler = Handler(Looper.getMainLooper())
    private val recoveryHandler = Handler(Looper.getMainLooper())
    private val responseRefresh = GoogleWebResponseRefreshCoordinator(
        requestSnapshot = { pageAdapter?.requestSnapshot() },
        schedule = { task, delayMs -> handler.postDelayed(task, delayMs) },
        cancel = handler::removeCallbacks,
    )
    private val recovery = WebChatSessionRecoveryCoordinator(
        schedule = { task, delayMs -> recoveryHandler.postDelayed(task, delayMs) },
        cancel = recoveryHandler::removeCallbacks,
        retry = ::reloadRestorablePage,
        onExhausted = {
            updateState(State.ERROR, "Google 网页 AI 自动重连多次失败，请检查网络后重试")
        },
    )
    private var webView: WebView? = null
    private var pageAdapter: GoogleWebPageAdapter? = null
    private var latestSnapshot: ChatGptWebSnapshot? = snapshotStore.restore()
    private var latestSnapshotPath: String? = latestSnapshot?.url?.let(conversationStore::currentPath)
    private var activePath: String? = null
    private var awaitingNewConversationBoundary = false
    private var state = State.IDLE
    private var reloadAfterPause = false

    fun activate() {
        latestSnapshot?.let(onSnapshot)
        onConversationIndexChanged(conversationIndex())
        recovery.activate()
        ensureInitialized()
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
        resumeRecovery()
    }

    fun deactivate() = pauseSession()

    fun onHostResumed() {
        recovery.activate()
        webView?.onResume()
        pageAdapter?.onHostResumed(webView?.url)
        resumeRecovery()
    }

    fun retryConnection(): Boolean = recovery.retryNow()

    fun onHostPaused() = pauseSession()

    fun currentSnapshot(): ChatGptWebSnapshot? = latestSnapshot

    fun state(): State = state

    fun canSend(): Boolean = WebChatSendContextPolicy.allows(
        state == State.READY, latestSnapshot, conversationNavigation.hasPending(),
        conversationNavigation.selectedPath(activePath),
        latestSnapshot?.url?.let(conversationStore::currentPath),
    )

    fun sendPrompt(prompt: String): Boolean {
        val snapshot = latestSnapshot ?: return false
        if (!canSend()) return false
        responseRefresh.onSendStarted(prompt)
        pageAdapter?.sendPrompt(prompt, snapshot.draft) ?: return false
        return true
    }

    fun onSubmissionObserved() = responseRefresh.onSendConfirmed()

    fun stopGeneration() = pageAdapter?.stopGeneration()

    fun startNewConversation() {
        responseRefresh.stop()
        conversationNavigation.cancel()
        activePath = null
        awaitingNewConversationBoundary = true
        pageAdapter?.startNewConversation()
    }

    fun currentConversationPath(): String? = conversationNavigation.selectedPath(activePath)

    fun currentOfficialUrl(): String? = activePath
        ?.let(conversationStore::restorableUrl)
        ?: GoogleWebNavigationPolicy.sanitizeRestorableUrl(latestSnapshot?.url)

    fun conversationIndex(): ChatGptWebConversationIndexState = conversationStore.index(activePath)

    fun requestConversationIndex(): Boolean {
        onConversationIndexChanged(conversationIndex())
        pageAdapter?.requestSnapshot()
        return webView != null
    }

    fun openConversation(path: String): Boolean {
        val url = conversationStore.restorableUrl(path) ?: return false
        if (!GoogleWebNavigationPolicy.supportsAiMode(url)) return false
        val view = webView ?: return false
        responseRefresh.stop()
        awaitingNewConversationBoundary = false
        conversationNavigation.beginOpen(path, url)
        activePath = path
        latestSnapshotPath = path
        latestSnapshot = conversationSnapshotStore.restore(path)?.takeIf { cached ->
            GoogleWebNavigationPolicy.sanitizeRestorableUrl(cached.url) == url
        }
            ?: GoogleWebSnapshotPresentation.loading(latestSnapshot, url)
        latestSnapshot?.let(onSnapshot)
        onConversationIndexChanged(conversationIndex())
        updateState(State.LOADING)
        view.loadUrl(url)
        return true
    }

    fun openProject(path: String): Boolean = false

    fun createLocalProject(title: String): Boolean {
        val changed = conversationStore.createProject(title)
        if (changed) onConversationIndexChanged(conversationIndex())
        return changed
    }

    fun assignConversationToLocalProject(path: String, projectId: String?): Boolean {
        val changed = conversationStore.assignConversation(path, projectId)
        if (changed) onConversationIndexChanged(conversationIndex())
        return changed
    }

    fun destroy() {
        recovery.dispose()
        recoveryHandler.removeCallbacksAndMessages(null)
        responseRefresh.stop()
        conversationNavigation.cancel()
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
                if (!recovery.isActive()) {
                    reloadAfterPause = true
                    adapter.onHostPaused()
                } else if (GoogleWebNavigationPolicy.supportsAiMode(url)) {
                    recovery.onNavigationStarted()
                } else {
                    recovery.onTerminal()
                }
            },
            onPageReady = { url ->
                cookieManager.flush()
                if (!recovery.isActive()) {
                    reloadAfterPause = false
                    adapter.onHostPaused()
                } else {
                    adapter.onPageReady(url)
                    if (GoogleWebNavigationPolicy.supportsAiMode(url)) {
                        recovery.onPageFinished()
                    } else {
                        recovery.onTerminal()
                        updateState(State.ERROR, "Google 官方页面未进入 AI 模式，请打开官方页确认")
                    }
                }
            },
            onBlockedNavigation = { hostName ->
                recovery.onTerminal()
                updateState(State.ERROR, "Google 官方导航被拦截：$hostName")
            },
            onPageError = { detail ->
                updateState(State.ERROR, detail)
                if (GoogleWebNavigationPolicy.supportsAiMode(view.url)) {
                    recovery.onFailure()
                } else {
                    recovery.onTerminal()
                }
            },
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
            if (!recovery.isActive()) {
                reloadAfterPause = true
                return@prepare
            }
            status.error?.let {
                updateState(State.ERROR, it)
                recovery.onFailure()
                return@prepare
            }
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
                val rawSnapshot = event.value
                if (!conversationNavigation.shouldAccept(rawSnapshot.url)) return
                when (GoogleWebNewConversationPolicy.transition(
                    awaitingBoundary = awaitingNewConversationBoundary,
                    previous = latestSnapshot,
                    incoming = rawSnapshot,
                )) {
                    GoogleWebNewConversationTransition.IGNORE_STALE -> return
                    GoogleWebNewConversationTransition.START_NEW -> {
                        awaitingNewConversationBoundary = false
                        latestSnapshot = null
                        latestSnapshotPath = null
                    }
                    GoogleWebNewConversationTransition.CONTINUE_CURRENT -> Unit
                }
                val pageTitle = event.value.title.trim().takeUnless {
                    it.equals("Google", ignoreCase = true) ||
                        it.equals("Google AI 模式", ignoreCase = true)
                }
                val title = rawSnapshot.messages.lastOrNull { it.role == "user" }
                    ?.content
                    ?.takeIf(String::isNotBlank)
                    ?: pageTitle.orEmpty()
                val preferredPath = activePath ?: latestSnapshotPath
                val observedPath = if (rawSnapshot.messages.any { it.role == "user" }) {
                    conversationStore.observe(
                        rawSnapshot.url,
                        title,
                        LocalDate.now(),
                        preferredPath,
                    )
                } else {
                    conversationStore.currentPath(rawSnapshot.url)
                }
                val nextSnapshot = GoogleWebSnapshotMerger.merge(
                    previous = latestSnapshot,
                    incoming = rawSnapshot,
                    sameConversation = observedPath != null && observedPath == latestSnapshotPath,
                )
                latestSnapshot = nextSnapshot
                latestSnapshotPath = observedPath
                activePath = observedPath
                if (nextSnapshot.composerReady && !nextSnapshot.streaming) {
                    snapshotStore.save(nextSnapshot)
                    observedPath?.let { conversationSnapshotStore.save(it, nextSnapshot) }
                }
                GoogleWebNavigationPolicy.sanitizeRestorableUrl(rawSnapshot.url)?.let { url ->
                    preferences.edit().putString(KEY_LAST_URL, url).apply()
                }
                if (nextSnapshot.composerReady) {
                    recovery.onReady()
                    updateState(State.READY)
                } else {
                    updateState(State.LOADING)
                }
                val lastUserIndex = nextSnapshot.messages.indexOfLast { it.role == "user" }
                responseRefresh.onSnapshot(
                    latestUserPrompt = nextSnapshot.messages.getOrNull(lastUserIndex)?.content,
                    assistantObserved = lastUserIndex >= 0 && nextSnapshot.messages
                        .drop(lastUserIndex + 1)
                        .any { it.role == "assistant" },
                    streaming = nextSnapshot.streaming,
                )
                onConversationIndexChanged(conversationIndex())
                onSnapshot(nextSnapshot)
            }
            is ChatGptWebEvent.CommandResult -> {
                if (event.action == DOM_DIAGNOSTICS_ACTION) {
                    Log.i(DOM_DIAGNOSTICS_TAG, event.detail.take(160))
                    return
                }
                onCommandResult(event)
                if (event.action == "send_prompt") {
                    if (event.ok) responseRefresh.onSendConfirmed() else responseRefresh.stop()
                }
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
            GoogleWebPageAdapter.State.UNSUPPORTED -> {
                recovery.onTerminal()
                updateState(State.ERROR, "当前 WebView 不支持 Google 网页 AI 语义桥接")
            }
            GoogleWebPageAdapter.State.CONNECTING -> updateState(State.LOADING)
            GoogleWebPageAdapter.State.WEB_ONLY -> Unit
        }
    }

    private fun updateState(next: State, detail: String? = null) {
        if (next == State.ERROR) responseRefresh.stop()
        state = next
        onStateChanged(next, detail)
    }

    private fun pauseSession() {
        recovery.deactivate()
        responseRefresh.stop()
        handler.removeCallbacksAndMessages(null)
        pageAdapter?.onHostPaused()
        webView?.let { view ->
            if (state == State.LOADING && GoogleWebNavigationPolicy.supportsAiMode(view.url)) {
                view.stopLoading()
                reloadAfterPause = true
            }
            cookieManager.flush()
            view.onPause()
        }
    }

    private fun resumeRecovery() {
        when {
            reloadAfterPause -> {
                reloadAfterPause = false
                recovery.retryNow()
            }
            state == State.ERROR && GoogleWebNavigationPolicy.supportsAiMode(webView?.url) ->
                recovery.onFailure()
            state == State.LOADING && GoogleWebNavigationPolicy.supportsAiMode(webView?.url) ->
                recovery.onPageFinished()
        }
    }

    private fun reloadRestorablePage(): Boolean {
        val view = webView ?: return false
        val restored = GoogleWebNavigationPolicy.sanitizeRestorableUrl(view.url)
            ?: GoogleWebNavigationPolicy.sanitizeRestorableUrl(
                preferences.getString(KEY_LAST_URL, null),
            )
            ?: GoogleWebNavigationPolicy.START_URL
        view.stopLoading()
        updateState(State.LOADING)
        view.loadUrl(restored)
        return true
    }

    private companion object {
        const val PREFERENCES_NAME = "google_web_session"
        const val KEY_LAST_URL = "last_ai_mode_url"
        const val DOM_DIAGNOSTICS_ACTION = "dom_diagnostics"
        const val DOM_DIAGNOSTICS_TAG = "ElonGoogleWebDom"
    }
}

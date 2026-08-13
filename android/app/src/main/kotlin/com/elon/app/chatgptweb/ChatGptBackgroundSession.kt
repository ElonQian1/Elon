package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.graphics.Color
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.CookieManager
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.PendingAttachment

internal class ChatGptBackgroundSession(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val onSnapshot: (ChatGptWebSnapshot) -> Unit,
    private val onStateChanged: (State, String?) -> Unit,
    private val onComposerOptions: (List<ChatGptWebComposerOption>) -> Unit,
    private val onCommandResult: (ChatGptWebEvent.CommandResult) -> Unit,
    private val onAttachmentSendChanged: (ChatGptWebAttachmentSendUpdate) -> Unit,
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
    private val uploadStager = ChatGptWebUploadStager(activity)
    private val attachmentHandler = Handler(Looper.getMainLooper())
    private var webView: WebView? = null
    private var pageAdapter: ChatGptWebPageAdapter? = null
    private var touchDispatcher: ChatGptWebTouchDispatcher? = null
    private var latestSnapshot: ChatGptWebSnapshot? = null
    private var state = State.IDLE
    private var queuedUploadUris = emptyList<Uri>()
    private var attachmentSendTracker: ChatGptWebAttachmentSendTracker? = null
    private var lastAttachmentSendPhase = ATTACHMENT_PHASE_IDLE

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
        lastAttachmentSendPhase = ATTACHMENT_PHASE_IDLE
        adapter.sendPrompt(prompt, snapshot.draft)
        return true
    }

    fun sendAttachments(prompt: String, attachments: List<PendingAttachment>): Boolean {
        val adapter = pageAdapter ?: return false
        val snapshot = latestSnapshot ?: return false
        if (!canSend() || attachments.isEmpty() || attachmentSendTracker != null) return false
        val uris = runCatching { uploadStager.stage(attachments) }.getOrNull() ?: return false
        attachmentSendTracker = ChatGptWebAttachmentSendTracker.begin(prompt, attachments.size, snapshot)
        lastAttachmentSendPhase = ChatGptWebAttachmentSendTracker.Phase.UPLOADING.wireValue
        queuedUploadUris = uris
        onAttachmentSendChanged(
            ChatGptWebAttachmentSendUpdate(
                phase = ChatGptWebAttachmentSendTracker.Phase.UPLOADING.wireValue,
                attachmentCount = attachments.size,
            ),
        )
        scheduleAttachmentTimeout(attachmentSendTracker ?: return false)
        adapter.requestAttachmentUpload()
        return true
    }

    fun attachmentSendPhase(): String = attachmentSendTracker?.phase?.wireValue ?: lastAttachmentSendPhase

    fun pendingAttachmentCount(): Int = attachmentSendTracker?.localAttachmentCount ?: 0

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
        attachmentHandler.removeCallbacksAndMessages(null)
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
        cancelAttachmentSend()
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
            webChromeClient = object : WebChromeClient() {
                override fun onShowFileChooser(
                    webView: WebView,
                    filePathCallback: ValueCallback<Array<Uri>>,
                    fileChooserParams: FileChooserParams,
                ): Boolean {
                    val values = queuedUploadUris
                    queuedUploadUris = emptyList()
                    filePathCallback.onReceiveValue(values.takeIf { it.isNotEmpty() }?.toTypedArray())
                    if (values.isEmpty()) failAttachmentSend("附件请求已失效，请重新选择。")
                    return true
                }
            }
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
                processAttachmentSnapshot(snapshot)
                onSnapshot(snapshot)
            }
            is ChatGptWebEvent.ComposerControls -> {
                if (event.section == "model") onComposerOptions(event.options)
            }
            is ChatGptWebEvent.CommandResult -> {
                onCommandResult(event)
                processAttachmentCommandResult(event)
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

    private fun processAttachmentSnapshot(snapshot: ChatGptWebSnapshot) {
        val tracker = attachmentSendTracker ?: return
        when (val observation = tracker.observe(snapshot)) {
            ChatGptWebAttachmentSendTracker.Observation.Wait -> Unit
            ChatGptWebAttachmentSendTracker.Observation.SendPrompt -> {
                lastAttachmentSendPhase = tracker.phase.wireValue
                onAttachmentSendChanged(
                    ChatGptWebAttachmentSendUpdate(
                        phase = tracker.phase.wireValue,
                        attachmentCount = tracker.localAttachmentCount,
                    ),
                )
                pageAdapter?.sendPrompt(tracker.prompt, snapshot.draft)
                    ?: failAttachmentSend("官网发送入口尚未就绪。")
            }
            is ChatGptWebAttachmentSendTracker.Observation.Complete -> {
                attachmentHandler.removeCallbacksAndMessages(null)
                attachmentSendTracker = null
                queuedUploadUris = emptyList()
                lastAttachmentSendPhase = ATTACHMENT_PHASE_COMPLETED
                onAttachmentSendChanged(
                    ChatGptWebAttachmentSendUpdate(
                        phase = ATTACHMENT_PHASE_COMPLETED,
                        attachmentCount = tracker.localAttachmentCount,
                        userMessageId = observation.userMessageId,
                    ),
                )
            }
            is ChatGptWebAttachmentSendTracker.Observation.Failed -> failAttachmentSend(observation.detail)
        }
    }

    private fun processAttachmentCommandResult(event: ChatGptWebEvent.CommandResult) {
        if (attachmentSendTracker == null || event.ok) return
        if (event.action == "request_attachment_upload" || event.action == "send_prompt") {
            failAttachmentSend(event.detail.ifBlank { "官网附件操作失败，请重试。" })
        }
    }

    private fun failAttachmentSend(detail: String) {
        val tracker = attachmentSendTracker ?: return
        attachmentHandler.removeCallbacksAndMessages(null)
        latestSnapshot?.let(tracker::uploadedAttachmentIds)?.forEach { id ->
            pageAdapter?.removeAttachment(id)
        }
        tracker.markSendFailed()
        queuedUploadUris = emptyList()
        lastAttachmentSendPhase = tracker.phase.wireValue
        onAttachmentSendChanged(
            ChatGptWebAttachmentSendUpdate(
                phase = tracker.phase.wireValue,
                attachmentCount = tracker.localAttachmentCount,
                detail = detail,
            ),
        )
        attachmentSendTracker = null
    }

    private fun cancelAttachmentSend() {
        attachmentHandler.removeCallbacksAndMessages(null)
        queuedUploadUris = emptyList()
        attachmentSendTracker = null
        lastAttachmentSendPhase = ATTACHMENT_PHASE_IDLE
    }

    private fun scheduleAttachmentTimeout(tracker: ChatGptWebAttachmentSendTracker) {
        attachmentHandler.removeCallbacksAndMessages(null)
        attachmentHandler.postDelayed(
            {
                if (attachmentSendTracker === tracker) {
                    failAttachmentSend("附件上传超时，请检查网络后重试。")
                }
            },
            ATTACHMENT_TIMEOUT_MS,
        )
    }

    private companion object {
        const val ATTACHMENT_PHASE_IDLE = "idle"
        const val ATTACHMENT_PHASE_COMPLETED = "completed"
        const val ATTACHMENT_TIMEOUT_MS = 120_000L
    }
}

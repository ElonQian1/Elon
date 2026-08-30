package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.webkit.CookieManager
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.PendingAttachment
import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatManagedRealtimeVoiceState
import com.elon.app.WebChatPendingSendState
import com.elon.app.WebChatSendCoordinator
import com.elon.app.WebChatSessionRecoveryCoordinator
import com.elon.app.WebChatSocialMcpPort
import java.time.LocalDate

internal class ChatGptBackgroundSession(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val onSnapshot: (ChatGptWebSnapshot) -> Unit,
    private val onStateChanged: (State, String?) -> Unit,
    private val onComposerOptions: (List<ChatGptWebComposerOption>) -> Unit,
    private val onCommandResult: (ChatGptWebEvent.CommandResult, ChatGptWebSendReceipt?) -> Unit,
    private val onSendTerminalTimeout: (WebChatPendingSendState.TimeoutResult) -> Unit,
    private val onAttachmentSendChanged: (ChatGptWebAttachmentSendUpdate) -> Unit,
    private val onConversationIndexChanged: (ChatGptWebConversationIndexState) -> Unit,
    private val audioPermissionController: ChatGptWebAudioPermissionController,
    private val onRealtimeVoiceTranscript: (ChatGptWebNativeVoiceTranscriptEvent) -> Unit = {},
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
    private val proxyController = ChatGptWebProxyController(activity)
    private val uploadStager = ChatGptWebUploadStager(activity)
    private val conversationHistoryStore = ChatGptConversationHistoryStore(activity)
    private val conversationNavigation = ChatGptConversationNavigationCoordinator(activity)
    private val snapshotStore = WebChatSnapshotStore(activity, "chatgpt")
    private val restoredConversationHistory = conversationHistoryStore.restore()
    private val conversationDirectory = ChatGptConversationDirectory(restoredConversationHistory)
    private val restoredSnapshot = snapshotStore.restore()
    private val sessionContinuity = ChatGptWebSessionContinuity(
        initialAuthenticated = restoredSnapshot?.authenticated == true ||
            restoredConversationHistory != null,
    )
    private val observedMcpState = ChatGptWebObservedState(restoredConversationHistory)
    private val verificationEvidenceStore = ChatGptWebVerificationEvidenceStore(activity.applicationContext)
    private val sendHandler = Handler(Looper.getMainLooper())
    private val conversationRefreshHandler = Handler(Looper.getMainLooper())
    private val composerOptionHandler = Handler(Looper.getMainLooper())
    private val surfaceMode: ChatGptWebSurfaceModeController by lazy(LazyThreadSafetyMode.NONE) {
        ChatGptWebSurfaceModeController(
            { webView }, { pageAdapter }, { webExecution.interactionRequested() }, ::ensureInitialized,
        )
    }
    private val composerOptionInteraction by lazy(LazyThreadSafetyMode.NONE) {
        ChatGptComposerOptionInteraction({ webView }, { pageAdapter }, surfaceMode::isSkin, composerOptionHandler)
    }
    private val sessionContinuityHandler = Handler(Looper.getMainLooper())
    private val recoveryHandler = Handler(Looper.getMainLooper())
    private val conversationRefresh = ChatGptConversationRefreshCoordinator(
        dispatch = ::dispatchConversationIndexRequest,
        schedule = { task, delayMs -> conversationRefreshHandler.postDelayed(task, delayMs) },
        cancel = conversationRefreshHandler::removeCallbacks,
    )
    private val composerOptionRequests = ChatGptComposerOptionRequestCoordinator(
        dismissMenu = composerOptionInteraction::dismiss,
        dispatchRequest = composerOptionInteraction::dispatch,
        collectOptions = { section ->
            if (section == "model") pageAdapter?.collectModelOptions()
            else pageAdapter?.collectComposerTools()
        },
        schedule = { task, delayMs -> composerOptionHandler.postDelayed(task, delayMs) },
        cancel = composerOptionHandler::removeCallbacks,
        prepareSection = observedMcpState::beginComposerRequest,
        failSuperseded = { requestId, section ->
            observedMcpState.failCommand(
                requestId,
                chatGptComposerListAction(section),
                "composer_request_superseded",
            )
        },
    )
    private val recovery = WebChatSessionRecoveryCoordinator(
        schedule = { task, delayMs -> recoveryHandler.postDelayed(task, delayMs) },
        cancel = recoveryHandler::removeCallbacks,
        retry = ::reloadRestorablePage,
        repair = { repairChatGptCurrentDocument(webView, pageAdapter, webExecution::interactionRequested) },
        onExhausted = { updateState(State.ERROR, "网页 AI 自动重连多次失败，请检查网络后重试") },
    )
    private var webView: WebView? = null
    private var pageAdapter: ChatGptWebPageAdapter? = null
    private var touchDispatcher: ChatGptWebTouchDispatcher? = null
    private var latestSnapshot: ChatGptWebSnapshot? = restoredSnapshot
    private val realtimeVoiceRecovery = ChatGptRealtimeVoiceConversationRecovery(restoredSnapshot)
    private var warmSessionAvailable = restoredSnapshot != null
    private var latestUiManifest: ChatGptWebUiManifest? = null
    private var latestBridgeState = ChatGptWebPageAdapter.State.WEB_ONLY
    private var state = State.IDLE
    private var forceConversationRefreshAfterVoice = false
    private var requestedConversationProjectId: String? = null
    private var loadPendingAfterPause = false
    private val realtimeVoiceBacking: ChatGptRealtimeVoiceBackingController by
        lazy(LazyThreadSafetyMode.NONE) {
            ChatGptRealtimeVoiceBackingController(
                activity.applicationContext,
                ::ensureInitialized, { webView }, surfaceMode, ::invokeRealtimeVoiceControl,
                { webExecution.interactionRequested() },
                { pageAdapter?.requestConversationRefresh() },
                { pageAdapter?.requestSnapshot() },
                { task, delay -> recoveryHandler.postDelayed(task, delay) },
                realtimeVoiceRecovery::revision,
                realtimeVoiceRecovery::recoveredSince,
                onRealtimeVoiceTranscript,
            )
        }
    private val sendOwner = ChatGptWebSendOwner(
        transport = chatGptOfficialPageSendTransport(
            pageAdapter = { pageAdapter },
            snapshot = { latestSnapshot },
            ready = ::canSend,
        ),
        snapshot = { latestSnapshot },
        stageUploads = { attachments -> uploadStager.stage(attachments) },
        requestAttachmentUpload = {
            pageAdapter?.let {
                it.requestAttachmentUpload()
                true
            } ?: false
        },
        removeAttachment = { id -> pageAdapter?.removeAttachment(id) },
        postDelayed = { task, delayMs -> sendHandler.postDelayed(task, delayMs) },
        removeCallbacks = sendHandler::removeCallbacks,
        onTerminalTimeout = onSendTerminalTimeout,
        onAttachmentChanged = onAttachmentSendChanged,
        onSendStateChanged = { latestSnapshot?.let(onSnapshot) },
    )
    private val webExecution: ChatGptBackgroundExecutionController =
        chatGptBackgroundExecutionController({ webView }) {
            surfaceMode.isSkin() || realtimeVoiceBacking.isActive() ||
                latestSnapshot?.streaming == true ||
                conversationNavigation.hasPending() || sendOwner.hasAttachmentSend()
        }
    private val touchRequestHandler by lazy(LazyThreadSafetyMode.NONE) {
        ChatGptWebTouchRequestHandler(
            { webView }, { pageAdapter }, { touchDispatcher }, webExecution::interactionRequested,
            composerOptionRequests::dismiss,
            { composerOptionRequests.scheduleCollection("model") },
            { composerOptionRequests.scheduleCollection("tools") },
            { onStateChanged(state, "官网控件操作未就绪") },
        )
    }
    private val newConversationRecovery = ChatGptNewConversationRecoveryCoordinator(
        webView = { webView },
        navigationActive = conversationNavigation::isNavigating,
        loading = { state == State.LOADING },
        composerReady = { latestSnapshot?.composerReady == true },
        interactionRequested = webExecution::interactionRequested,
    )
    private val navigationActions by lazy(LazyThreadSafetyMode.NONE) {
        ChatGptSessionNavigationActions(
            { state == State.READY }, { state == State.IDLE || state == State.LOADING },
            { latestBridgeState == ChatGptWebPageAdapter.State.READY }, { pageAdapter != null },
            { pageAdapter?.startNewConversation() }, { path -> pageAdapter?.openConversation(path) },
            { path ->
                if (!conversationDirectory.requestProject(path)) false else {
                    onConversationIndexChanged(conversationIndex())
                    pageAdapter?.openProject(path)
                    true
                }
            },
            { latestSnapshot }, { snapshot -> latestSnapshot = snapshot; onSnapshot(snapshot) },
            { updateState(State.LOADING) }, ::ensureInitialized,
            newConversationRecovery::cancel, newConversationRecovery::schedule,
            conversationNavigation,
        )
    }

    fun activate() {
        latestSnapshot?.let(onSnapshot)
        onConversationIndexChanged(conversationIndex())
        recovery.activate()
        ensureInitialized()
        webExecution.hostResumed()
        pageAdapter?.onHostResumed(webView?.url)
        resumeRecovery()
    }
    fun deactivate() {
        if (realtimeVoiceBacking.isActive()) {
            webExecution.interactionRequested()
            cookieManager.flush()
        } else {
            pauseSession()
        }
    }
    fun onHostResumed() {
        if (webView == null) return
        recovery.activate()
        webExecution.hostResumed()
        realtimeVoiceBacking.restoreAfterHostResume()
        pageAdapter?.onHostResumed(webView?.url)
        resumeRecovery()
    }
    fun retryGuestAccess(): Boolean {
        val view = webView ?: return false
        view.stopLoading()
        updateState(State.LOADING)
        webExecution.interactionRequested()
        view.loadUrl(ChatGptWebNavigationPolicy.START_URL)
        return true
    }
    fun retryConnection(): Boolean = recovery.retryNow()
    fun onHostPaused() = if (realtimeVoiceBacking.isActive()) cookieManager.flush() else pauseSession()
    fun currentSnapshot(): ChatGptWebSnapshot? = latestSnapshot
    fun warmSessionAvailable(): Boolean = warmSessionAvailable
    fun conversationNavigationActive(): Boolean = conversationNavigation.isNavigating()
    fun conversationIndex(): ChatGptWebConversationIndexState = conversationDirectory.index()
    fun requestConversationIndex(projectId: String? = null): Boolean {
        requestedConversationProjectId = ChatGptWebConversationPath.canonicalProjectId(projectId)
        return conversationRefresh.requestAfterCurrent()
    }

    private fun dispatchConversationIndexRequest(): Boolean {
        val adapter = pageAdapter ?: return false
        if (state != State.READY) return false
        val refreshRequest = conversationDirectory.beginRefresh(requestedConversationProjectId)
        requestedConversationProjectId = null
        onConversationIndexChanged(conversationIndex())
        adapter.listConversations(
            projectHints = refreshRequest.projectHints,
            scopeProjectId = refreshRequest.scopeProjectId,
        )
        return true
    }

    fun state(): State = state

    fun presentationMode(): ChatGptWebPresentationMode = surfaceMode.mode()
    fun selectPresentationMode(mode: ChatGptWebPresentationMode): Boolean = surfaceMode.select(mode)

    fun canSend(): Boolean = WebChatSendContextPolicy.allows(state == State.READY, latestSnapshot, conversationNavigation.hasPending(), currentConversationPath(), currentConversationPath())

    fun sendReady(): Boolean = sendOwner.isReady()
    fun pendingSendPrompt(): String? = sendOwner.prompt()
    fun pendingSendStatus(): String? = sendOwner.status()
    fun pendingSendRequiresOfficialConfirmation(): Boolean =
        sendOwner.requiresOfficialConfirmation()
    fun dispatchSocialPrompt(prompt: String) = sendOwner.dispatchSocial(prompt)
    fun pauseSendWatchdog() = sendOwner.pauseWatchdog()
    fun clearPendingSend() = sendOwner.clear()

    fun sendAttachments(prompt: String, attachments: List<PendingAttachment>): Boolean {
        return sendOwner.beginAttachments(prompt, attachments)
    }

    fun attachmentSendPhase(): String = sendOwner.attachmentSendPhase()

    fun pendingAttachmentCount(): Int = sendOwner.pendingAttachmentCount()

    fun requestModelOptions(): Boolean {
        if (state != State.READY) return false
        return composerOptionRequests.request("model")
    }

    fun dismissComposerOptions() = composerOptionRequests.dismiss()

    fun selectModel(id: String) {
        pageAdapter?.selectModelOption(id)
    }

    fun stopGeneration() {
        pageAdapter?.stopGeneration()
    }

    fun startNewConversation(): Boolean = navigationActions.startNewConversation()

    fun currentConversationPath(): String? = ChatGptWebConversationPath.fromUrl(latestSnapshot?.url)

    fun currentOfficialUrl(): String? = latestSnapshot?.url
        ?.takeIf(ChatGptWebNavigationPolicy::allows)

    fun openConversation(path: String): Boolean = navigationActions.openConversation(path)

    fun openProject(path: String): Boolean = navigationActions.openProject(path)

    fun createMcpPort(
        inputText: () -> String,
        setInputText: (String) -> Unit,
        copyMessage: (String) -> ChatGptClipboardMetadata,
        selectMode: (ChatGptWebPresentationMode) -> Unit,
        revealMessage: (String, Int?, String) -> Boolean,
    ): WebChatSocialMcpPort {
        ensureInitialized()
        val adapter = checkNotNull(pageAdapter) { "ChatGPT background session is not active" }
        val commands = ChatGptWebMcpCommandAdapter(
            pageAdapter = adapter,
            sendInputAction = { requestId ->
                val result = sendOwner.dispatchMcp(inputText().trim(), requestId)
                if (result.outcome != WebChatSendCoordinator.DispatchOutcome.DISPATCHED) {
                    observedMcpState.failCommand(
                        requestId,
                        "send_prompt",
                        when (result.outcome) {
                            WebChatSendCoordinator.DispatchOutcome.BUSY -> "send_busy"
                            WebChatSendCoordinator.DispatchOutcome.NOT_READY ->
                                "send_not_ready"
                            WebChatSendCoordinator.DispatchOutcome.REJECTED ->
                                "send_rejected"
                            WebChatSendCoordinator.DispatchOutcome.DISPATCHED ->
                                "send_dispatched"
                        },
                    )
                }
            },
            invokeControlAction = adapter::invokeUiControl,
            startDictationAction = { requestId ->
                audioPermissionController.runWithMicrophone(
                    action = { adapter.startDictation(requestId) },
                    onPermissionDenied = {
                        observedMcpState.failCommand(
                            requestId,
                            "start_dictation",
                            "microphone_permission_denied",
                        )
                    },
                )
            },
            requestComposerOptionsAction = { section, requestId ->
                composerOptionRequests.request(section, requestId)
            },
            dismissComposerOptionsAction = composerOptionRequests::dismiss,
        )
        val officialPort = ChatGptWebMcpActions(
            snapshot = { latestSnapshot },
            uiManifest = { latestUiManifest },
            observedState = observedMcpState::snapshot,
            beginCommand = observedMcpState::beginCommand,
            bridgeState = { latestBridgeState },
            mode = ::presentationMode,
            inputText = inputText,
            audioPermissionState = audioPermissionController::snapshot,
            verificationEvidence = verificationEvidenceStore::snapshot,
            recordVerificationCases = verificationEvidenceStore::record,
            setInputText = setInputText,
            copyMessage = copyMessage,
            commands = commands,
            refresh = { webView?.reload() },
            selectMode = selectMode,
            revealMessage = revealMessage,
        )
        return ChatGptWebNativeVoiceResearchMcpPort(
            delegate = officialPort,
            startNative = realtimeVoiceBacking::beginNativePrivateVoiceResearch,
            muteNative = realtimeVoiceBacking::muteNativePrivateVoiceResearch,
            stopNative = { realtimeVoiceBacking.end(gracefulExit = true) },
            currentState = realtimeVoiceBacking::nativePrivateVoiceState,
        )
    }

    fun createConsumerPort(mcpPort: WebChatSocialMcpPort): WebChatConsumerPort =
        ChatGptWebConsumerPortAdapter(
            snapshot = { latestSnapshot },
            uiManifest = { latestUiManifest },
            observedState = observedMcpState::snapshot,
            executeControl = mcpPort::control,
        )

    private fun invokeRealtimeVoiceControl(): Boolean {
        val adapter = pageAdapter ?: return false
        val control = ChatGptRealtimeVoicePolicy.resolve(latestUiManifest) ?: return false
        adapter.invokeUiControl(control.id)
        webExecution.interactionRequested()
        return true
    }

    fun beginRealtimeVoiceBacking(): Boolean = realtimeVoiceBacking.begin()
    fun startManagedRealtimeVoice(): Boolean = realtimeVoiceBacking.startManagedRealtimeVoice()
    fun managedRealtimeVoiceState(): WebChatManagedRealtimeVoiceState =
        realtimeVoiceBacking.managedRealtimeVoiceState()
    fun setManagedRealtimeVoiceMuted(muted: Boolean): Boolean =
        realtimeVoiceBacking.setManagedRealtimeVoiceMuted(muted)
    fun realtimeVoiceActive(): Boolean = realtimeVoiceBacking.isActive()
    fun endRealtimeVoiceBacking(gracefulExit: Boolean) {
        if (!realtimeVoiceBacking.isActive()) return
        if (latestSnapshot?.capabilities?.supports(ChatGptWebCapabilityId.CONVERSATION_LIST) == true) {
            conversationDirectory.markRefreshing()
            forceConversationRefreshAfterVoice = true
            onConversationIndexChanged(conversationIndex())
        }
        realtimeVoiceBacking.end(gracefulExit)
    }

    fun destroy() {
        realtimeVoiceBacking.release()
        composerOptionInteraction.release()
        recovery.dispose()
        recoveryHandler.removeCallbacksAndMessages(null)
        newConversationRecovery.cancel()
        conversationNavigation.clear()
        navigationActions.clearDeferred()
        composerOptionRequests.reset()
        pageAdapter?.dispose()
        sendOwner.dispose()
        sendHandler.removeCallbacksAndMessages(null)
        conversationRefresh.reset()
        forceConversationRefreshAfterVoice = false
        conversationRefreshHandler.removeCallbacksAndMessages(null)
        composerOptionHandler.removeCallbacksAndMessages(null)
        sessionContinuityHandler.removeCallbacksAndMessages(null)
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
        latestUiManifest = null
        latestBridgeState = ChatGptWebPageAdapter.State.WEB_ONLY
        updateState(State.IDLE)
    }

    private fun ensureInitialized() {
        if (webView != null) return
        WebView.setWebContentsDebuggingEnabled(false)
        val view = createChatGptBackgroundWebView(
            activity, audioPermissionController, recovery::onNavigationProgress,
        ) { callback ->
            val values = sendOwner.consumeQueuedUploadUris()
            callback.onReceiveValue(values.takeIf { it.isNotEmpty() }?.toTypedArray())
            if (values.isEmpty()) sendOwner.failFileChooser("附件请求已失效，请重新选择。")
        }
        surfaceMode.attach(view)
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(view, true)
        val adapter = ChatGptWebPageAdapter(
            context = activity,
            webView = view,
            onEvent = ::handleEvent,
            onStateChanged = ::handleAdapterState,
            onDocumentChanged = ::handleDocumentChanged,
            onWebExecutionRequested = { webExecution.interactionRequested() },
        )
        view.webViewClient = ChatGptWebViewClient(
            onPageStarted = { url ->
                composerOptionInteraction.release()
                conversationRefresh.reset()
                composerOptionRequests.reset()
                sessionContinuityHandler.removeCallbacksAndMessages(null)
                adapter.onPageStarted(url)
                updateState(State.LOADING)
                if (!recovery.isActive()) {
                    adapter.onHostPaused()
                } else if (ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) recovery.onNavigationStarted()
                else recovery.onTerminal()
            },
            onPageReady = { url ->
                cookieManager.flush()
                sessionRestorer.onPageReady(url)
                if (!recovery.isActive()) {
                    adapter.onHostPaused()
                } else {
                    adapter.onPageReady(url)
                    if (ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) recovery.onPageFinished()
                    else recovery.onTerminal()
                }
            },
            onBlockedNavigation = { hostName ->
                navigationActions.clearDeferred()
                recovery.onTerminal()
                updateState(State.ERROR, "官网导航被拦截：$hostName")
            },
            onPageError = { detail ->
                navigationActions.clearDeferred()
                updateState(State.ERROR, detail)
                if (ChatGptWebNavigationPolicy.supportsEnhancedMode(view.url)) recovery.onFailure() else recovery.onTerminal()
            },
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
        webExecution.webViewAttached()
        pageAdapter = adapter
        touchDispatcher = ChatGptWebTouchDispatcher(view)
        adapter.install()
        surfaceMode.apply()
        proxyController.prepare { status ->
            if (activity.isFinishing || activity.isDestroyed || webView !== view) return@prepare
            if (!recovery.isActive()) { loadPendingAfterPause = true; return@prepare }
            status.error?.let {
                updateState(State.ERROR, it)
                recovery.onFailure()
                return@prepare
            }
            updateState(State.LOADING)
            webExecution.interactionRequested()
            view.loadUrl(ChatGptWebNavigationPolicy.restorableStartUrl(sessionRestorer.restoreUrl()))
        }
    }

    private fun handleEvent(event: ChatGptWebEvent) {
        if (ChatGptWebPrivateResearchEventRecorder.record(event)) return
        observedMcpState.accept(event)
        when (event) {
            is ChatGptWebEvent.Snapshot -> {
                if (!conversationNavigation.shouldAccept(event.value)) return
                val reconciliation = sessionContinuity.reconcileWithDecision(event.value)
                val previous = latestSnapshot
                val previousIdentity = ChatGptWebConversationPath.fromUrl(previous?.url)
                    ?.let(ChatGptWebConversationPath::identity)
                val incomingIdentity = ChatGptWebConversationPath.fromUrl(reconciliation.snapshot.url)
                    ?.let(ChatGptWebConversationPath::identity)
                val merged = WebChatSnapshotWindowMerger.merge(
                    previous = previous,
                    incoming = reconciliation.snapshot,
                    sameConversation = previousIdentity != null && previousIdentity == incomingIdentity,
                )
                val snapshot = ChatGptWebTransientComposerReadiness.reconcile(
                    previous = previous,
                    incoming = merged,
                    composerInteractionActive = ChatGptWebTransientComposerReadiness.interactionActive(
                        composerOptionRequests.isActive(), observedMcpState.snapshot().commandRequests,
                    ),
                )
                scheduleSessionContinuityRecheck(reconciliation.recheckAfterMs)
                if (reconciliation.clearConversationHistory) clearConversationHistory()
                latestSnapshot = snapshot
                realtimeVoiceRecovery.accept(snapshot)
                ChatGptWebConversationPath.fromUrl(snapshot.url)?.let {
                    conversationDirectory.observeCurrent(snapshot, LocalDate.now())
                    conversationDirectory.save(conversationHistoryStore)
                    onConversationIndexChanged(conversationIndex())
                }
                when {
                    ChatGptWebAccessPolicy.requiresLogin(snapshot) -> {
                        navigationActions.clearDeferred()
                        newConversationRecovery.cancel()
                        conversationNavigation.complete()
                        snapshotStore.clear()
                        warmSessionAvailable = false
                        pageAdapter?.markLoginRequired()
                        recovery.onTerminal()
                        updateState(State.LOGIN_REQUIRED)
                    }
                    ChatGptWebAccessPolicy.canChat(snapshot) -> {
                        newConversationRecovery.cancel()
                        conversationNavigation.complete()
                        if (!snapshot.streaming) {
                            snapshotStore.save(snapshot)
                            ChatGptWebConversationPath.fromUrl(snapshot.url)?.let { path ->
                                conversationNavigation.save(path, snapshot)
                            }
                        }
                        warmSessionAvailable = true
                        pageAdapter?.markReady()
                        recovery.onReady()
                        updateState(State.READY)
                        if (
                            forceConversationRefreshAfterVoice &&
                            snapshot.capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST)
                        ) {
                            forceConversationRefreshAfterVoice = false
                            conversationRefresh.requestAfterCurrent()
                        } else if (
                            snapshot.capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST) &&
                            conversationDirectory.needsProjectRefresh(snapshot.url)
                        ) {
                            conversationRefresh.requestAfterCurrent()
                        } else if (
                            snapshot.capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST) &&
                            conversationDirectory.needsOfficialRefresh()
                        ) {
                            conversationRefresh.requestIfIdle()
                        }
                    }
                    else -> updateState(State.LOADING)
                }
                sendOwner.observeSnapshot(snapshot)
                onSnapshot(snapshot)
                if (state == State.READY) navigationActions.onSessionReady()
            }
            is ChatGptWebEvent.ComposerControls -> {
                composerOptionInteraction.release()
                composerOptionRequests.complete(event.section)
                if (event.section == "model") onComposerOptions(event.options)
            }
            is ChatGptWebEvent.CommandResult -> {
                if (event.action == "dismiss_composer_menu") composerOptionRequests.onMenuDismissed()
                chatGptComposerSectionForAction(event.action)?.let { section ->
                    composerOptionInteraction.release()
                    composerOptionRequests.complete(section)
                }
                onCommandResult(event, sendOwner.acceptCommandResult(event))
                if (event.ok) {
                    pageAdapter?.requestSnapshot()
                } else {
                    if (event.action == "open_conversation" || event.action == "new_conversation") {
                        newConversationRecovery.cancel()
                        conversationNavigation.restoreAfterFailure(event.action)?.let { previous ->
                            latestSnapshot = previous
                            onSnapshot(previous)
                            updateState(when {
                                ChatGptWebAccessPolicy.requiresLogin(previous) -> State.LOGIN_REQUIRED
                                ChatGptWebAccessPolicy.canChat(previous) -> State.READY
                                else -> State.LOADING
                            })
                        }
                    }
                    if (event.action == "list_conversations") {
                        conversationDirectory.failRefresh()
                        onConversationIndexChanged(conversationIndex())
                        conversationRefresh.onFailed()
                    }
                    onStateChanged(
                        state,
                        ChatGptWebPrivateTextReceiptPolicy.userDetail(event)
                            .ifBlank { "官网操作失败" },
                    )
                }
            }
            is ChatGptWebEvent.ConversationList -> {
                conversationRefresh.onSucceeded()
                conversationDirectory.accept(event)
                conversationDirectory.save(conversationHistoryStore)
                onConversationIndexChanged(conversationIndex())
            }
            is ChatGptWebEvent.UiManifest -> {
                latestUiManifest = event.value
                realtimeVoiceBacking.onUiManifestAvailable()
                if (ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(latestSnapshot, event.value)) {
                    pageAdapter?.markReady()
                }
            }
            is ChatGptWebEvent.AdapterReady,
            is ChatGptWebEvent.FeatureNavigation -> Unit
            is ChatGptWebEvent.WebTouchRequest -> touchRequestHandler.handle(event)
        }
        webExecution.activitySettled()
    }

    private fun handleDocumentChanged(document: com.elon.app.WebBridgeDocumentSession.Snapshot) {
        if (document.pageGeneration > observedMcpState.snapshot().pageGeneration) {
            latestSnapshot = latestSnapshot?.let(ChatGptWebSnapshotPresentation::revalidating)
            latestSnapshot?.let(onSnapshot)
            latestUiManifest = null
        }
        observedMcpState.updateDocument(document)
    }

    private fun scheduleSessionContinuityRecheck(delayMs: Long?) {
        sessionContinuityHandler.removeCallbacksAndMessages(null)
        delayMs ?: return
        sessionContinuityHandler.postDelayed({
            val reconciliation = sessionContinuity.confirmPendingLoginEvidence() ?: return@postDelayed
            reconciliation.recheckAfterMs?.let {
                scheduleSessionContinuityRecheck(it)
                return@postDelayed
            }
            if (reconciliation.clearConversationHistory) clearConversationHistory()
            latestSnapshot = reconciliation.snapshot
            snapshotStore.clear()
            pageAdapter?.markLoginRequired()
            recovery.onTerminal()
            updateState(State.LOGIN_REQUIRED)
            onSnapshot(reconciliation.snapshot)
        }, delayMs)
    }

    private fun clearConversationHistory() {
        conversationDirectory.clear()
        conversationHistoryStore.clear()
        observedMcpState.clearConversationHistory()
        onConversationIndexChanged(conversationIndex())
    }

    private fun handleAdapterState(adapterState: ChatGptWebPageAdapter.State) {
        latestBridgeState = adapterState
        if (adapterState == ChatGptWebPageAdapter.State.READY) {
            navigationActions.onBridgeReady()
        } else if (adapterState == ChatGptWebPageAdapter.State.UNSUPPORTED) {
            recovery.onTerminal()
            updateState(State.ERROR, "当前 WebView 不支持网页 AI 语义桥接")
        }
    }

    private fun pauseSession() {
        webExecution.hostPaused()
        composerOptionInteraction.release()
        recovery.deactivate()
        conversationRefresh.reset()
        composerOptionRequests.reset()
        sessionContinuityHandler.removeCallbacksAndMessages(null)
        pageAdapter?.onHostPaused()
        cookieManager.flush()
    }

    private fun resumeRecovery() {
        val view = webView ?: return
        when {
            loadPendingAfterPause -> {
                loadPendingAfterPause = false
                recovery.retryNow()
            }
            state == State.ERROR && ChatGptWebNavigationPolicy.supportsEnhancedMode(view.url) -> recovery.onFailure()
            state == State.LOADING && ChatGptWebNavigationPolicy.supportsEnhancedMode(view.url) -> {
                if (view.progress >= 100) recovery.onPageFinished() else recovery.onNavigationStarted()
            }
        }
    }

    private fun reloadRestorablePage(): Boolean {
        val view = webView ?: return false
        val savedUrl = sequenceOf(latestSnapshot?.url, sessionRestorer.restoreUrl())
            .filterNotNull()
            .firstOrNull(ChatGptWebNavigationPolicy::supportsEnhancedMode)
            ?: ChatGptWebNavigationPolicy.START_URL
        view.stopLoading()
        updateState(State.LOADING)
        webExecution.interactionRequested()
        view.loadUrl(ChatGptWebNavigationPolicy.restorableStartUrl(savedUrl))
        return true
    }

    private fun updateState(next: State, detail: String? = null) {
        state = next
        onStateChanged(next, detail)
    }

}

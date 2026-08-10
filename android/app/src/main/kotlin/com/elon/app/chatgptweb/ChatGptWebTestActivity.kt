package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.webkit.CookieManager
import android.webkit.PermissionRequest
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebStorage
import android.webkit.WebView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.databinding.ActivityChatgptWebTestBinding
import com.elon.app.mcp.McpNativeControlBinding

class ChatGptWebTestActivity : AppCompatActivity() {
    private lateinit var binding: ActivityChatgptWebTestBinding
    private lateinit var pageAdapter: ChatGptWebPageAdapter
    private lateinit var nativeController: ChatGptNativeConversationController
    private lateinit var composerToolsController: ChatGptNativeComposerToolsController
    private lateinit var attachmentController: ChatGptNativeAttachmentController
    private lateinit var voiceController: ChatGptNativeVoiceController
    private lateinit var dictationSessionController: ChatGptWebDictationSessionController
    private lateinit var touchDispatcher: ChatGptWebTouchDispatcher
    private lateinit var conversationListController: ChatGptNativeConversationListController
    private lateinit var featureHubController: ChatGptNativeFeatureHubController
    private lateinit var adaptiveUiController: ChatGptNativeAdaptiveUiController
    private lateinit var overlayControlsController: ChatGptNativeOverlayControlsController
    private lateinit var officialOverlayController: ChatGptWebOfficialOverlayController
    private lateinit var loginController: ChatGptNativeLoginController
    private lateinit var googleAccountHintController: ChatGptGoogleAccountHintController
    private lateinit var modeController: ChatGptWebModeController
    private lateinit var proxyController: ChatGptWebProxyController
    private lateinit var fileChooserController: ChatGptWebFileChooserController
    private lateinit var audioPermissionController: ChatGptWebAudioPermissionController
    private var proxyStatus = ChatGptWebProxyStatus("手机网络")
    private var webAuthenticationStatus = ChatGptWebAuthenticationSupport.Status.UNSUPPORTED
    private var latestSnapshot: ChatGptWebSnapshot? = null
    private var latestUiManifest: ChatGptWebUiManifest? = null
    private val sessionContinuity = ChatGptWebSessionContinuity()
    private val observedMcpState = ChatGptWebObservedState()
    private var latestBridgeState = ChatGptWebPageAdapter.State.WEB_ONLY
    private var latestMode = ChatGptWebModeController.Mode.QUICK
    private val cookieManager: CookieManager by lazy { CookieManager.getInstance() }
    private val mcpCommandPort: ChatGptWebMcpCommandPort by lazy {
        object : ChatGptWebMcpCommandPort {
            override fun sendInput(requestId: String) = nativeController.submitFromMcp(requestId)

            override fun invokeControl(controlId: String, requestId: String) =
                invokeUiControl(controlId, requestId)

            override fun setControlText(controlId: String, text: String, requestId: String) =
                pageAdapter.setUiControlText(controlId, text, requestId)

            override fun setControlSelected(controlId: String, selected: Boolean, requestId: String) =
                pageAdapter.setUiControlSelected(controlId, selected, requestId)

            override fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String) =
                pageAdapter.selectUiControlChoice(controlId, choiceIndex, requestId)

            override fun setControlSlider(controlId: String, value: Double, requestId: String) =
                pageAdapter.setUiControlSlider(controlId, value, requestId)

            override fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String) =
                pageAdapter.setUiControlExpanded(controlId, expanded, requestId)

            override fun newConversation(requestId: String) = pageAdapter.startNewConversation(requestId)

            override fun stopGeneration(requestId: String) = pageAdapter.stopGeneration(requestId)

            override fun startDictation(requestId: String) {
                audioPermissionController.runWithMicrophone(
                    action = {
                        if (prepareDictationStart { pageAdapter.startDictation(requestId) } == null) {
                            observedMcpState.failCommand(
                                requestId,
                                "start_dictation",
                                "dictation_start_in_progress",
                            )
                        }
                    },
                    onPermissionDenied = {
                        observedMcpState.failCommand(
                            requestId,
                            "start_dictation",
                            "microphone_permission_denied",
                        )
                        showMicrophoneDenied()
                    },
                )
            }

            override fun cancelDictation(requestId: String) = pageAdapter.cancelDictation(requestId)

            override fun submitDictation(requestId: String) = pageAdapter.submitDictation(requestId)

            override fun removeAttachment(attachmentId: String, requestId: String) =
                pageAdapter.removeAttachment(attachmentId, requestId)

            override fun refreshControls(requestId: String) = pageAdapter.requestUiManifest(requestId)

            override fun listConversations(requestId: String) = pageAdapter.listConversations(requestId)

            override fun requestComposerOptions(section: String, requestId: String) =
                this@ChatGptWebTestActivity.requestComposerOptions(section, requestId)

            override fun selectComposerOption(section: String, optionId: String, requestId: String) {
                if (section == "model") pageAdapter.selectModelOption(optionId, requestId)
                else pageAdapter.selectComposerTool(optionId, requestId)
            }

            override fun requestFeatures(requestId: String) = pageAdapter.listFeatures(requestId)

            override fun selectFeature(featureId: String, requestId: String) =
                pageAdapter.selectFeature(featureId, requestId)

            override fun openConversation(path: String, requestId: String) =
                pageAdapter.openConversation(path, requestId)
        }
    }

    private val mcpActions: ChatGptWebMcpActions by lazy {
        ChatGptWebMcpActions(
            snapshot = { latestSnapshot },
            uiManifest = { latestUiManifest },
            observedState = observedMcpState::snapshot,
            beginCommand = observedMcpState::beginCommand,
            bridgeState = { latestBridgeState },
            mode = { latestMode },
            inputText = { binding.chatGptNativeComposer.text?.toString().orEmpty() },
            setInputText = { text ->
                binding.chatGptNativeComposer.setText(text)
                binding.chatGptNativeComposer.setSelection(text.length)
            },
            commands = mcpCommandPort,
            refresh = { binding.chatGptWebView.reload() },
            selectMode = modeController::select,
        )
    }

    private val mcpNativeControlBinding: McpNativeControlBinding by lazy {
        McpNativeControlBinding(
            activity = this,
            surfaceId = "chatgpt_web",
            uiStateProvider = mcpActions::uiState,
            controlHandler = mcpActions::control,
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityChatgptWebTestBinding.inflate(layoutInflater)
        setContentView(binding.root)
        proxyController = ChatGptWebProxyController(this)

        configureToolbar()
        configureWebView()
        configureEnhancedMode()
        applyProductEntryPresentation()
        configureBackNavigation()

        proxyController.prepare { status ->
            if (isFinishing || isDestroyed) return@prepare
            proxyStatus = status
            status.error?.let { Toast.makeText(this, it, Toast.LENGTH_LONG).show() }
            if (savedInstanceState == null || binding.chatGptWebView.restoreState(savedInstanceState) == null) {
                binding.chatGptWebView.loadUrl(ChatGptWebNavigationPolicy.START_URL)
            }
        }
    }

    private fun configureToolbar() {
        binding.chatGptWebBack.setOnClickListener { navigateBack() }
        binding.chatGptWebProxy.setOnClickListener {
            ChatGptWebProxyDialog.show(this, proxyController, ::applyProxyStatusAndReload)
        }
        binding.chatGptWebReload.setOnClickListener {
            proxyController.prepare(::applyProxyStatusAndReload)
        }
        binding.chatGptWebClearSession.setOnClickListener { confirmClearSession() }
    }

    private fun applyProxyStatusAndReload(status: ChatGptWebProxyStatus) {
        proxyStatus = status
        status.error?.let {
            Toast.makeText(this, it, Toast.LENGTH_LONG).show()
            return
        }
        binding.chatGptWebStatus.text = status.label
        binding.chatGptWebView.stopLoading()
        if (binding.chatGptWebView.url == null) {
            binding.chatGptWebView.loadUrl(ChatGptWebNavigationPolicy.START_URL)
        } else {
            binding.chatGptWebView.reload()
        }
    }

    private fun configureEnhancedMode() {
        nativeController = ChatGptNativeConversationController(
            messagesView = binding.chatGptNativeMessages,
            emptyView = binding.chatGptNativeEmpty,
            composer = binding.chatGptNativeComposer,
            sendButton = binding.chatGptNativeSend,
            stopButton = binding.chatGptNativeComposerStop,
            newConversationButton = binding.chatGptNativeNew,
            onSend = { prompt, expectedDraft, requestId ->
                pageAdapter.sendPrompt(prompt, expectedDraft, requestId)
            },
            onStop = { pageAdapter.stopGeneration() },
            onNewConversation = { pageAdapter.startNewConversation() },
            onRegenerate = { pageAdapter.regenerateResponse() },
            onInvokeControl = ::invokeUiControl,
            onOpenOfficialOutput = { modeController.select(ChatGptWebModeController.Mode.WEB) },
        )
        composerToolsController = ChatGptNativeComposerToolsController(
            activity = this,
            modelButton = binding.chatGptNativeModel,
            attachmentButton = binding.chatGptNativeAttachment,
            toolsButton = binding.chatGptNativeTools,
            onRequestModelOptions = { requestComposerOptions("model") },
            onRequestTools = { requestComposerOptions("tools") },
            onSelectModelOption = { pageAdapter.selectModelOption(it) },
            onSelectTool = { pageAdapter.selectComposerTool(it) },
            onDismissMenu = { pageAdapter.dismissComposerMenu() },
            onOpenOfficialModelSelector = { openOfficialComposerOptions("model") },
            onOpenOfficialTools = { openOfficialComposerOptions("tools") },
        )
        attachmentController = ChatGptNativeAttachmentController(
            scrollView = binding.chatGptNativeAttachmentsScroll,
            container = binding.chatGptNativeAttachments,
            onRemove = { pageAdapter.removeAttachment(it) },
        )
        voiceController = ChatGptNativeVoiceController(
            button = binding.chatGptNativeDictation,
            onToggle = {
                audioPermissionController.runWithMicrophone(::startDictation)
            },
        )
        conversationListController = ChatGptNativeConversationListController(
            activity = this,
            trigger = binding.chatGptNativeHistory,
            onRequestList = { pageAdapter.listConversations() },
            onOpenConversation = { path -> pageAdapter.openConversation(path) },
            onNewConversation = { pageAdapter.startNewConversation() },
        )
        featureHubController = ChatGptNativeFeatureHubController(
            activity = this,
            trigger = binding.chatGptNativeFeatures,
            onRequestFeatures = { pageAdapter.listFeatures() },
            onSelectFeature = { pageAdapter.selectFeature(it) },
            onDismissNavigation = { pageAdapter.dismissFeatures() },
            onOpenOfficial = { modeController.select(ChatGptWebModeController.Mode.WEB) },
        )
        adaptiveUiController = ChatGptNativeAdaptiveUiController(
            activity = this,
            titleView = binding.chatGptNativeTitle,
            headerActionsScroll = binding.chatGptNativeHeaderActionsScroll,
            headerActions = binding.chatGptNativeHeaderActions,
            suggestions = binding.chatGptNativeSuggestions,
            onSuggestionsVisibleChanged = nativeController::setSuggestionsVisible,
            onInvoke = ::invokeUiControl,
        )
        overlayControlsController = ChatGptNativeOverlayControlsController(
            activity = this,
            headerActionsScroll = binding.chatGptNativeHeaderActionsScroll,
            headerActions = binding.chatGptNativeHeaderActions,
            onInvoke = ::invokeUiControl,
            onSetText = { controlId, text -> pageAdapter.setUiControlText(controlId, text) },
            onSelectChoice = { controlId, choiceIndex ->
                pageAdapter.selectUiControlChoice(controlId, choiceIndex)
            },
            onSetSlider = { controlId, value -> pageAdapter.setUiControlSlider(controlId, value) },
        )
        pageAdapter = ChatGptWebPageAdapter(
            context = this,
            webView = binding.chatGptWebView,
            onEvent = ::handleBridgeEvent,
            onStateChanged = ::handleBridgeState,
        )
        touchDispatcher = ChatGptWebTouchDispatcher(binding.chatGptWebView)
        officialOverlayController = ChatGptWebOfficialOverlayController(
            dispatchEscape = {
                binding.chatGptWebView.dispatchKeyEvent(
                    KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ESCAPE),
                )
                binding.chatGptWebView.dispatchKeyEvent(
                    KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ESCAPE),
                )
            },
            schedule = { delayMs, action -> binding.chatGptWebView.postDelayed(action, delayMs) },
            refreshManifest = pageAdapter::requestUiManifest,
        )

        modeController = ChatGptWebModeController(
            window = window,
            root = binding.root,
            chromeViews = listOf(
                binding.chatGptWebToolbar,
                binding.chatGptWebStatus,
                binding.chatGptModeToggle,
            ),
            toggle = binding.chatGptModeToggle,
            quickButton = binding.chatGptModeQuick,
            webButton = binding.chatGptModeWeb,
            nativeButton = binding.chatGptModeNative,
            webView = binding.chatGptWebView,
            quickRoot = binding.chatGptQuickRoot,
            nativeRoot = binding.chatGptNativeRoot,
            onModeChanged = ::showMode,
        )
        dictationSessionController = ChatGptWebDictationSessionController(
            isNativeSelected = modeController::isNativeSelected,
            showOfficial = { modeController.select(ChatGptWebModeController.Mode.WEB) },
            restoreNative = { modeController.select(ChatGptWebModeController.Mode.NATIVE) },
            cancelOfficial = pageAdapter::cancelDictation,
            schedule = { delayMs, action -> binding.chatGptWebView.postDelayed(action, delayMs) },
        )

        loginController = ChatGptNativeLoginController(
            context = this,
            stageView = binding.chatGptQuickStage,
            elapsedView = binding.chatGptQuickElapsed,
            primaryButton = binding.chatGptQuickLogin,
            officialButton = binding.chatGptQuickOfficial,
            onOpenAuthentication = {
                modeController.select(ChatGptWebModeController.Mode.WEB)
                binding.chatGptWebView.stopLoading()
                binding.chatGptWebView.loadUrl(ChatGptWebNavigationPolicy.AUTH_URL)
            },
            onOpenOfficialPage = {
                modeController.select(ChatGptWebModeController.Mode.WEB)
                if (binding.chatGptWebView.url == null) {
                    binding.chatGptWebView.loadUrl(ChatGptWebNavigationPolicy.START_URL)
                }
            },
            onOpenNativeConversation = {
                if (binding.chatGptModeNative.isEnabled) {
                    modeController.select(ChatGptWebModeController.Mode.NATIVE)
                }
            },
        )
        googleAccountHintController = ChatGptGoogleAccountHintController(
            activity = this,
            accountButton = binding.chatGptQuickGoogle,
            onBeginAuthentication = { loginController.beginAuthentication() },
            onRequestGoogleProvider = { pageAdapter.startGoogleLogin() },
            onStatusMessage = ::showGoogleLoginStatus,
        )

        renderWebAuthenticationStatus()
        modeController.attach()
        // install() reports its initial state synchronously, so every callback consumer
        // must be initialized before the bridge is installed.
        pageAdapter.install()
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun configureWebView() {
        WebView.setWebContentsDebuggingEnabled(false)
        fileChooserController = ChatGptWebFileChooserController(this)
        audioPermissionController = ChatGptWebAudioPermissionController(this, ::showMicrophoneDenied)
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(binding.chatGptWebView, true)

        binding.chatGptWebView.apply {
            setBackgroundColor(Color.WHITE)
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_YES
            settings.apply {
                // ChatGPT requires JavaScript; top-level navigation remains domain-restricted.
                javaScriptEnabled = true
                domStorageEnabled = true
                allowFileAccess = false
                // Required for content:// URIs returned by the system picker. The picker
                // still grants access only to files explicitly selected by the user.
                allowContentAccess = true
                javaScriptCanOpenWindowsAutomatically = false
                setSupportMultipleWindows(false)
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
                builtInZoomControls = false
                displayZoomControls = false
            }
            webAuthenticationStatus = ChatGptWebAuthenticationSupport.configure(settings)
            webViewClient = ChatGptWebViewClient(
                onPageStarted = ::showLoading,
                onPageReady = ::showReady,
                onBlockedNavigation = ::showBlockedNavigation,
                onPageError = ::showError,
                rewriteAllowedMainFrameUrl = { url ->
                    if (::googleAccountHintController.isInitialized) {
                        googleAccountHintController.rewriteGoogleAuthorization(url)
                    } else {
                        null
                    }
                },
            )
            webChromeClient = object : WebChromeClient() {
                override fun onProgressChanged(view: WebView, newProgress: Int) {
                    binding.chatGptWebProgress.progress = newProgress
                    binding.chatGptWebProgress.visibility = if (newProgress < 100) View.VISIBLE else View.GONE
                }

                override fun onShowFileChooser(
                    webView: WebView,
                    filePathCallback: ValueCallback<Array<Uri>>,
                    fileChooserParams: FileChooserParams,
                ): Boolean = fileChooserController.show(webView, filePathCallback, fileChooserParams)

                override fun onPermissionRequest(request: PermissionRequest) {
                    runOnUiThread { audioPermissionController.handle(request) }
                }

                override fun onPermissionRequestCanceled(request: PermissionRequest) {
                    runOnUiThread { audioPermissionController.cancel(request) }
                }
            }
        }
    }

    private fun configureBackNavigation() {
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() = navigateBack()
            },
        )
    }

    private fun navigateBack() {
        if (dictationSessionController.handleBack()) return
        when (ChatGptWebBackNavigation.decide(
            latestUiManifest,
            binding.chatGptWebView.canGoBack(),
            modeController.isWebSelected(),
        )) {
            ChatGptWebBackNavigation.Action.DISMISS_OFFICIAL_OVERLAY -> officialOverlayController.dismissTop()
            ChatGptWebBackNavigation.Action.NAVIGATE_WEB_HISTORY -> binding.chatGptWebView.goBack()
            ChatGptWebBackNavigation.Action.EXIT_OFFICIAL_VIEW -> modeController.exitOfficialView()
            ChatGptWebBackNavigation.Action.FINISH_ACTIVITY -> finish()
        }
    }

    private fun requestComposerOptions(section: String) = requestComposerOptions(section, null)

    private fun requestComposerOptions(section: String, requestId: String?) {
        observedMcpState.beginComposerRequest(section)
        officialOverlayController.dismissAllThen {
            if (section == "model") pageAdapter.listModelOptions(requestId)
            else pageAdapter.listComposerTools(requestId)
        }
    }

    private fun startDictation() {
        prepareDictationStart(pageAdapter::startDictation)
    }

    private fun showMicrophoneDenied() {
        Toast.makeText(this, R.string.chatgpt_native_microphone_denied, Toast.LENGTH_LONG).show()
    }

    private fun invokeUiControl(id: String) = invokeUiControl(id, null)

    private fun invokeUiControl(id: String, requestId: String?) {
        if (latestUiManifest?.controls?.firstOrNull { it.id == id }?.semantic == ChatGptWebUiSemantics.DICTATION) {
            if (prepareDictationStart { pageAdapter.invokeUiControl(id, requestId) } == null && requestId != null) {
                observedMcpState.failCommand(
                    requestId,
                    "invoke_ui_control",
                    "dictation_start_in_progress",
                )
            }
            return
        }
        pageAdapter.invokeUiControl(id, requestId)
    }

    private fun prepareDictationStart(startOfficial: () -> Unit): Long? {
        val attempt = dictationSessionController.onStartRequested(startOfficial) ?: return null
        binding.chatGptWebView.postDelayed(
            { dictationSessionController.onStartTimedOut(attempt) },
            DICTATION_START_TIMEOUT_MS,
        )
        return attempt
    }

    private fun showLoading(url: String) {
        pageAdapter.onPageStarted(url)
        loginController.onPageStarted(url)
        binding.chatGptWebHost.text = ChatGptWebNavigationPolicy.displayHost(url)
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_text_secondary))
        binding.chatGptWebStatus.text = statusWithProxy(R.string.chatgpt_web_loading)
    }

    private fun showReady(url: String) {
        cookieManager.flush()
        loginController.onPageReady(url)
        binding.chatGptWebHost.text = ChatGptWebNavigationPolicy.displayHost(url)
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_success))
        binding.chatGptWebStatus.text = statusWithProxy(R.string.chatgpt_web_ready)
        pageAdapter.onPageReady(url)
        googleAccountHintController.onPageReady(url)
    }

    private fun handleBridgeEvent(event: ChatGptWebEvent) {
        observedMcpState.accept(event)
        when (event) {
            is ChatGptWebEvent.ConversationList -> conversationListController.render(event.conversations)
            is ChatGptWebEvent.ComposerControls -> composerToolsController.render(event)
            is ChatGptWebEvent.FeatureNavigation -> featureHubController.render(event.features)
            is ChatGptWebEvent.UiManifest -> {
                latestUiManifest = event.value
                adaptiveUiController.render(event.value)
                overlayControlsController.render(event.value)
                nativeController.renderUiManifest(event.value)
            }
            is ChatGptWebEvent.WebTouchRequest -> handleWebTouchRequest(event)
            is ChatGptWebEvent.Snapshot -> {
                val snapshot = sessionContinuity.reconcile(event.value)
                latestSnapshot = snapshot
                nativeController.render(snapshot)
                composerToolsController.render(snapshot)
                attachmentController.render(snapshot)
                voiceController.render(snapshot)
                dictationSessionController.onSnapshot(snapshot.dictationActive)
                conversationListController.renderCapabilities(snapshot.capabilities)
                featureHubController.renderCapabilities(snapshot.capabilities)
                if (
                    snapshot.authenticated &&
                    (
                        snapshot.composerReady || snapshot.messages.isNotEmpty() ||
                            snapshot.dictationActive || snapshot.pageKind == "feature"
                    )
                ) {
                    googleAccountHintController.onAuthenticated()
                    pageAdapter.markReady()
                    if (loginController.onAuthenticated() || modeController.isQuickSelected()) {
                        modeController.select(ChatGptWebModeController.Mode.NATIVE)
                    }
                } else {
                    pageAdapter.markLoginRequired()
                }
                if (snapshot.title.isNotBlank()) binding.chatGptWebHost.text = snapshot.title
            }
            is ChatGptWebEvent.CommandResult -> {
                if (!event.ok && event.action in DICTATION_START_ACTIONS) {
                    dictationSessionController.onStartFailed()
                }
                if (googleAccountHintController.onCommandResult(event)) return
                if (conversationListController.onCommandResult(event)) return
                if (featureHubController.onCommandResult(event)) return
                nativeController.onCommandResult(event)
                composerToolsController.onCommandResult(event)
                if (!event.ok) {
                    binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_project))
                    binding.chatGptWebStatus.text = event.detail.ifBlank { getString(R.string.chatgpt_native_command_failed) }
                    Toast.makeText(this, binding.chatGptWebStatus.text, Toast.LENGTH_LONG).show()
                } else {
                    pageAdapter.requestSnapshot()
                }
            }
        }
    }

    private fun handleWebTouchRequest(event: ChatGptWebEvent.WebTouchRequest) {
        touchDispatcher.dispatch(event) { dispatched ->
            if (!dispatched) {
                showError(getString(R.string.chatgpt_native_command_failed))
                return@dispatch
            }
            when (event.purpose) {
                "list_model_options" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::collectModelOptions,
                    COMPOSER_MENU_SETTLE_MS,
                )
                "list_composer_tools" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::collectComposerTools,
                    COMPOSER_MENU_SETTLE_MS,
                )
                "open_model_submenu" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::collectModelOptions,
                    COMPOSER_MENU_SETTLE_MS,
                )
                "open_composer_tools_submenu" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::collectComposerTools,
                    COMPOSER_MENU_SETTLE_MS,
                )
                "list_navigation" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::collectFeatures,
                    NAVIGATION_SETTLE_MS,
                )
                "select_model_option", "select_composer_tool", "remove_attachment", "start_dictation",
                "cancel_dictation", "submit_dictation" ->
                    binding.chatGptWebView.postDelayed(pageAdapter::requestSnapshot, COMPOSER_MENU_SETTLE_MS)
                "select_navigation" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::requestSnapshot,
                    NAVIGATION_SETTLE_MS,
                )
                "invoke_ui_control" -> binding.chatGptWebView.postDelayed(
                    pageAdapter::requestSnapshot,
                    ADAPTIVE_CONTROL_SETTLE_MS,
                )
            }
        }
    }

    private fun handleBridgeState(state: ChatGptWebPageAdapter.State) {
        latestBridgeState = state
        nativeController.setBridgeState(state)
        composerToolsController.setBridgeState(state)
        voiceController.setBridgeState(state)
        conversationListController.setBridgeState(state)
        featureHubController.setBridgeState(state)
        binding.chatGptModeNative.isEnabled = state == ChatGptWebPageAdapter.State.READY
        binding.chatGptModeNative.alpha = if (binding.chatGptModeNative.isEnabled) 1f else 0.48f
        if (
            state in setOf(ChatGptWebPageAdapter.State.WEB_ONLY, ChatGptWebPageAdapter.State.UNSUPPORTED) &&
            modeController.isNativeSelected()
        ) {
            modeController.select(ChatGptWebModeController.Mode.WEB)
        }
    }

    private fun showMode(mode: ChatGptWebModeController.Mode) {
        latestMode = mode
        binding.chatGptWebStatus.setTextColor(
            getColor(
                if (mode == ChatGptWebModeController.Mode.NATIVE) {
                    R.color.elon_status_success
                } else {
                    R.color.elon_text_secondary
                },
            ),
        )
        binding.chatGptWebStatus.text = statusWithProxy(
            when (mode) {
                ChatGptWebModeController.Mode.QUICK -> R.string.chatgpt_quick_status
                ChatGptWebModeController.Mode.NATIVE -> R.string.chatgpt_native_active
                ChatGptWebModeController.Mode.WEB -> R.string.chatgpt_web_ready
            },
        )
    }

    private fun openOfficialComposerOptions(section: String) {
        modeController.select(ChatGptWebModeController.Mode.WEB)
        binding.chatGptWebView.postDelayed({
            if (section == "model") pageAdapter.listModelOptions()
            else pageAdapter.listComposerTools()
        }, COMPOSER_MENU_SETTLE_MS)
    }

    private fun statusWithProxy(messageResource: Int): String =
        "${getString(messageResource)} · ${proxyStatus.label}"

    private fun renderWebAuthenticationStatus() {
        val enabled = webAuthenticationStatus == ChatGptWebAuthenticationSupport.Status.ENABLED
        binding.chatGptQuickWebAuthentication.setText(
            if (enabled) {
                R.string.chatgpt_web_authentication_ready
            } else {
                R.string.chatgpt_web_authentication_unavailable
            },
        )
        binding.chatGptQuickWebAuthentication.setTextColor(
            getColor(if (enabled) R.color.elon_status_success else R.color.elon_text_tertiary),
        )
    }

    private fun showBlockedNavigation(host: String) {
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_project))
        binding.chatGptWebStatus.text = getString(R.string.chatgpt_web_blocked_host, host)
        Toast.makeText(this, R.string.chatgpt_web_blocked_toast, Toast.LENGTH_LONG).show()
    }

    private fun showError(message: String) {
        loginController.onPageError()
        binding.chatGptWebProgress.visibility = View.GONE
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_danger))
        binding.chatGptWebStatus.text = "$message · ${proxyStatus.label}".take(160)
    }

    private fun showGoogleLoginStatus(messageResource: Int) {
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_project))
        binding.chatGptWebStatus.setText(messageResource)
        Toast.makeText(this, messageResource, Toast.LENGTH_LONG).show()
    }

    private fun confirmClearSession() {
        AlertDialog.Builder(this)
            .setTitle(R.string.chatgpt_web_clear)
            .setMessage(R.string.chatgpt_web_clear_message)
            .setNegativeButton(R.string.chatgpt_web_cancel, null)
            .setPositiveButton(R.string.chatgpt_web_clear_confirm) { _, _ -> clearSession() }
            .show()
    }

    private fun clearSession() {
        dictationSessionController.reset()
        modeController.select(ChatGptWebModeController.Mode.QUICK)
        loginController.reset()
        googleAccountHintController.reset()
        sessionContinuity.clear()
        cookieManager.removeAllCookies {
            cookieManager.flush()
            WebStorage.getInstance().deleteAllData()
            binding.chatGptWebView.apply {
                clearCache(true)
                clearHistory()
                clearSslPreferences()
                loadUrl(ChatGptWebNavigationPolicy.START_URL)
            }
            Toast.makeText(this, R.string.chatgpt_web_clear_success, Toast.LENGTH_SHORT).show()
        }
    }

    override fun onResume() {
        super.onResume()
        mcpNativeControlBinding.register()
        binding.chatGptWebView.onResume()
        pageAdapter.onHostResumed(binding.chatGptWebView.url)
    }

    override fun onPause() {
        cookieManager.flush()
        binding.chatGptWebView.onPause()
        super.onPause()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        binding.chatGptWebView.saveState(outState)
        super.onSaveInstanceState(outState)
    }

    override fun onDestroy() {
        mcpNativeControlBinding.unregister()
        googleAccountHintController.dispose()
        loginController.dispose()
        conversationListController.dispose()
        featureHubController.dispose()
        overlayControlsController.dispose()
        officialOverlayController.dispose()
        dictationSessionController.reset()
        composerToolsController.dispose()
        fileChooserController.dispose()
        audioPermissionController.dispose()
        pageAdapter.dispose()
        binding.chatGptWebView.apply {
            stopLoading()
            webChromeClient = null
            destroy()
        }
        super.onDestroy()
    }

    private fun applyProductEntryPresentation() {
        if (!intent.getBooleanExtra(EXTRA_PRODUCT_ENTRY, false)) return
        binding.chatGptWebToolbar.visibility = View.GONE
        binding.chatGptWebStatus.visibility = View.GONE
        binding.chatGptModeToggle.visibility = View.GONE
    }

    companion object {
        private const val EXTRA_PRODUCT_ENTRY = "chatgpt_product_entry"
        const val COMPOSER_MENU_SETTLE_MS = 320L
        const val NAVIGATION_SETTLE_MS = 420L
        const val ADAPTIVE_CONTROL_SETTLE_MS = 360L
        const val DICTATION_START_TIMEOUT_MS = 20_000L
        val DICTATION_START_ACTIONS = setOf("start_dictation", "invoke_ui_control")

        fun createProductIntent(context: Context): Intent =
            Intent(context, ChatGptWebTestActivity::class.java).putExtra(EXTRA_PRODUCT_ENTRY, true)
    }
}

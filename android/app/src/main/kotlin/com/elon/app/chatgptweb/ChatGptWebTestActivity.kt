package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
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

class ChatGptWebTestActivity : AppCompatActivity() {
    private lateinit var binding: ActivityChatgptWebTestBinding
    private lateinit var pageAdapter: ChatGptWebPageAdapter
    private lateinit var nativeController: ChatGptNativeConversationController
    private lateinit var composerToolsController: ChatGptNativeComposerToolsController
    private lateinit var attachmentController: ChatGptNativeAttachmentController
    private lateinit var voiceController: ChatGptNativeVoiceController
    private lateinit var touchDispatcher: ChatGptWebTouchDispatcher
    private lateinit var conversationListController: ChatGptNativeConversationListController
    private lateinit var loginController: ChatGptNativeLoginController
    private lateinit var googleAccountHintController: ChatGptGoogleAccountHintController
    private lateinit var modeController: ChatGptWebModeController
    private lateinit var proxyController: ChatGptWebProxyController
    private lateinit var fileChooserController: ChatGptWebFileChooserController
    private lateinit var audioPermissionController: ChatGptWebAudioPermissionController
    private var proxyStatus = ChatGptWebProxyStatus("手机网络")
    private var webAuthenticationStatus = ChatGptWebAuthenticationSupport.Status.UNSUPPORTED
    private val cookieManager: CookieManager by lazy { CookieManager.getInstance() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityChatgptWebTestBinding.inflate(layoutInflater)
        setContentView(binding.root)
        proxyController = ChatGptWebProxyController(this)

        configureToolbar()
        configureWebView()
        configureEnhancedMode()
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
            stopButton = binding.chatGptNativeStop,
            newConversationButton = binding.chatGptNativeNew,
            onSend = { prompt, expectedDraft -> pageAdapter.sendPrompt(prompt, expectedDraft) },
            onStop = { pageAdapter.stopGeneration() },
            onNewConversation = { pageAdapter.startNewConversation() },
            onRegenerate = { pageAdapter.regenerateResponse() },
        )
        composerToolsController = ChatGptNativeComposerToolsController(
            activity = this,
            modelButton = binding.chatGptNativeModel,
            attachmentButton = binding.chatGptNativeAttachment,
            toolsButton = binding.chatGptNativeTools,
            onRequestModelOptions = { pageAdapter.listModelOptions() },
            onRequestTools = { pageAdapter.listComposerTools() },
            onSelectModelOption = { pageAdapter.selectModelOption(it) },
            onSelectTool = { pageAdapter.selectComposerTool(it) },
            onDismissMenu = { pageAdapter.dismissComposerMenu() },
            onOpenOfficialModelSelector = { modeController.select(ChatGptWebModeController.Mode.WEB) },
            onOpenOfficialTools = { modeController.select(ChatGptWebModeController.Mode.WEB) },
        )
        attachmentController = ChatGptNativeAttachmentController(
            scrollView = binding.chatGptNativeAttachmentsScroll,
            container = binding.chatGptNativeAttachments,
            onRemove = { pageAdapter.removeAttachment(it) },
        )
        voiceController = ChatGptNativeVoiceController(
            button = binding.chatGptNativeDictation,
            onToggle = {
                audioPermissionController.runWithMicrophone(pageAdapter::startDictation)
            },
        )
        conversationListController = ChatGptNativeConversationListController(
            activity = this,
            trigger = binding.chatGptNativeHistory,
            onRequestList = { pageAdapter.listConversations() },
            onOpenConversation = { path -> pageAdapter.openConversation(path) },
        )
        pageAdapter = ChatGptWebPageAdapter(
            context = this,
            webView = binding.chatGptWebView,
            onEvent = ::handleBridgeEvent,
            onStateChanged = ::handleBridgeState,
        )
        touchDispatcher = ChatGptWebTouchDispatcher(binding.chatGptWebView)

        modeController = ChatGptWebModeController(
            window = window,
            root = binding.root,
            toggle = binding.chatGptModeToggle,
            quickButton = binding.chatGptModeQuick,
            webButton = binding.chatGptModeWeb,
            nativeButton = binding.chatGptModeNative,
            webView = binding.chatGptWebView,
            quickRoot = binding.chatGptQuickRoot,
            nativeRoot = binding.chatGptNativeRoot,
            onModeChanged = ::showMode,
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
        audioPermissionController = ChatGptWebAudioPermissionController(this) {
            Toast.makeText(this, R.string.chatgpt_native_microphone_denied, Toast.LENGTH_LONG).show()
        }
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
        if (binding.chatGptWebView.canGoBack()) {
            binding.chatGptWebView.goBack()
        } else {
            finish()
        }
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
        when (event) {
            is ChatGptWebEvent.ConversationList -> conversationListController.render(event.conversations)
            is ChatGptWebEvent.ComposerControls -> composerToolsController.render(event)
            is ChatGptWebEvent.WebTouchRequest -> handleWebTouchRequest(event)
            is ChatGptWebEvent.Snapshot -> {
                nativeController.render(event.value)
                composerToolsController.render(event.value)
                attachmentController.render(event.value)
                voiceController.render(event.value)
                conversationListController.renderCapabilities(event.value.capabilities)
                if (
                    event.value.authenticated &&
                    (event.value.composerReady || event.value.messages.isNotEmpty())
                ) {
                    googleAccountHintController.onAuthenticated()
                    pageAdapter.markReady()
                    if (loginController.onAuthenticated() || modeController.isQuickSelected()) {
                        modeController.select(ChatGptWebModeController.Mode.NATIVE)
                    }
                } else {
                    pageAdapter.markLoginRequired()
                }
                if (event.value.title.isNotBlank()) binding.chatGptWebHost.text = event.value.title
            }
            is ChatGptWebEvent.CommandResult -> {
                if (googleAccountHintController.onCommandResult(event)) return
                if (conversationListController.onCommandResult(event)) return
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
                "select_model_option", "select_composer_tool", "remove_attachment", "start_dictation" ->
                    binding.chatGptWebView.postDelayed(pageAdapter::requestSnapshot, COMPOSER_MENU_SETTLE_MS)
            }
        }
    }

    private fun handleBridgeState(state: ChatGptWebPageAdapter.State) {
        nativeController.setBridgeState(state)
        composerToolsController.setBridgeState(state)
        voiceController.setBridgeState(state)
        conversationListController.setBridgeState(state)
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
        modeController.select(ChatGptWebModeController.Mode.QUICK)
        loginController.reset()
        googleAccountHintController.reset()
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
        googleAccountHintController.dispose()
        loginController.dispose()
        conversationListController.dispose()
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

    private companion object {
        const val COMPOSER_MENU_SETTLE_MS = 320L
    }
}

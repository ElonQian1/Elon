package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.webkit.CookieManager
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
    private val cookieManager: CookieManager by lazy { CookieManager.getInstance() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityChatgptWebTestBinding.inflate(layoutInflater)
        setContentView(binding.root)

        configureToolbar()
        configureWebView()
        configureEnhancedMode()
        configureBackNavigation()

        if (savedInstanceState == null || binding.chatGptWebView.restoreState(savedInstanceState) == null) {
            binding.chatGptWebView.loadUrl(ChatGptWebNavigationPolicy.START_URL)
        }
    }

    private fun configureToolbar() {
        binding.chatGptWebBack.setOnClickListener { navigateBack() }
        binding.chatGptWebReload.setOnClickListener { binding.chatGptWebView.reload() }
        binding.chatGptWebClearSession.setOnClickListener { confirmClearSession() }
    }

    private fun configureEnhancedMode() {
        nativeController = ChatGptNativeConversationController(
            messagesView = binding.chatGptNativeMessages,
            emptyView = binding.chatGptNativeEmpty,
            composer = binding.chatGptNativeComposer,
            sendButton = binding.chatGptNativeSend,
            stopButton = binding.chatGptNativeStop,
            newConversationButton = binding.chatGptNativeNew,
            onSend = { prompt -> pageAdapter.sendPrompt(prompt) },
            onStop = { pageAdapter.stopGeneration() },
            onNewConversation = { pageAdapter.startNewConversation() },
        )
        pageAdapter = ChatGptWebPageAdapter(
            context = this,
            webView = binding.chatGptWebView,
            onEvent = ::handleBridgeEvent,
            onStateChanged = ::handleBridgeState,
        )
        pageAdapter.install()

        binding.chatGptModeToggle.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) showNativeMode(checkedId == R.id.chatGptModeNative)
        }
        binding.chatGptModeWeb.isChecked = true
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun configureWebView() {
        WebView.setWebContentsDebuggingEnabled(false)
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
                allowContentAccess = false
                javaScriptCanOpenWindowsAutomatically = false
                setSupportMultipleWindows(false)
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
                builtInZoomControls = false
                displayZoomControls = false
            }
            webViewClient = ChatGptWebViewClient(
                onPageStarted = ::showLoading,
                onPageReady = ::showReady,
                onBlockedNavigation = ::showBlockedNavigation,
                onPageError = ::showError,
            )
            webChromeClient = object : WebChromeClient() {
                override fun onProgressChanged(view: WebView, newProgress: Int) {
                    binding.chatGptWebProgress.progress = newProgress
                    binding.chatGptWebProgress.visibility = if (newProgress < 100) View.VISIBLE else View.GONE
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
        binding.chatGptWebHost.text = ChatGptWebNavigationPolicy.displayHost(url)
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_text_secondary))
        binding.chatGptWebStatus.setText(R.string.chatgpt_web_loading)
    }

    private fun showReady(url: String) {
        cookieManager.flush()
        binding.chatGptWebHost.text = ChatGptWebNavigationPolicy.displayHost(url)
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_success))
        binding.chatGptWebStatus.setText(R.string.chatgpt_web_ready)
        pageAdapter.onPageReady(url)
    }

    private fun handleBridgeEvent(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.Snapshot -> {
                nativeController.render(event.value)
                if (event.value.composerReady || event.value.messages.isNotEmpty()) {
                    pageAdapter.markReady()
                }
                if (event.value.title.isNotBlank()) binding.chatGptWebHost.text = event.value.title
            }
            is ChatGptWebEvent.CommandResult -> {
                if (!event.ok) {
                    nativeController.restoreComposerState()
                    binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_project))
                    binding.chatGptWebStatus.text = event.detail.ifBlank { getString(R.string.chatgpt_native_command_failed) }
                    Toast.makeText(this, binding.chatGptWebStatus.text, Toast.LENGTH_LONG).show()
                } else {
                    pageAdapter.requestSnapshot()
                }
            }
        }
    }

    private fun handleBridgeState(state: ChatGptWebPageAdapter.State) {
        nativeController.setBridgeState(state)
        binding.chatGptModeNative.isEnabled = state == ChatGptWebPageAdapter.State.READY
        binding.chatGptModeNative.alpha = if (binding.chatGptModeNative.isEnabled) 1f else 0.48f
        if (state != ChatGptWebPageAdapter.State.READY && binding.chatGptModeNative.isChecked) {
            binding.chatGptModeWeb.isChecked = true
        }
    }

    private fun showNativeMode(nativeMode: Boolean) {
        binding.chatGptWebView.visibility = if (nativeMode) View.INVISIBLE else View.VISIBLE
        binding.chatGptNativeRoot.visibility = if (nativeMode) View.VISIBLE else View.GONE
        binding.chatGptWebStatus.setText(
            if (nativeMode) R.string.chatgpt_native_active else R.string.chatgpt_web_ready,
        )
    }

    private fun showBlockedNavigation(host: String) {
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_project))
        binding.chatGptWebStatus.text = getString(R.string.chatgpt_web_blocked_host, host)
        Toast.makeText(this, R.string.chatgpt_web_blocked_toast, Toast.LENGTH_LONG).show()
    }

    private fun showError(message: String) {
        binding.chatGptWebProgress.visibility = View.GONE
        binding.chatGptWebStatus.setTextColor(getColor(R.color.elon_status_danger))
        binding.chatGptWebStatus.text = message.take(120)
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
        binding.chatGptModeWeb.isChecked = true
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
        pageAdapter.dispose()
        binding.chatGptWebView.apply {
            stopLoading()
            webChromeClient = null
            destroy()
        }
        super.onDestroy()
    }
}

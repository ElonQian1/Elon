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
    private val cookieManager: CookieManager by lazy { CookieManager.getInstance() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityChatgptWebTestBinding.inflate(layoutInflater)
        setContentView(binding.root)

        configureToolbar()
        configureWebView()
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
        binding.chatGptWebView.apply {
            stopLoading()
            webChromeClient = null
            destroy()
        }
        super.onDestroy()
    }
}

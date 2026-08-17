package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.webkit.CookieManager
import android.webkit.PermissionRequest
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R

/** Full-screen official-page fallback for login and capabilities not mirrored natively. */
class ChatGptWebOfficialActivity : AppCompatActivity() {
    private lateinit var webView: WebView
    private lateinit var progress: ProgressBar
    private lateinit var fileChooserController: ChatGptWebFileChooserController
    private lateinit var audioPermissionController: ChatGptWebAudioPermissionController
    private lateinit var proxyController: ChatGptWebProxyController
    private val cookieManager: CookieManager by lazy { CookieManager.getInstance() }
    private val sessionRestorer by lazy { ChatGptWebSessionRestorer(this) }

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WebView.setWebContentsDebuggingEnabled(false)
        fileChooserController = ChatGptWebFileChooserController(this)
        audioPermissionController = ChatGptWebAudioPermissionController(
            this,
            ::showMicrophoneDenied,
        )
        proxyController = ChatGptWebProxyController(this)

        webView = WebView(this).apply {
            setBackgroundColor(Color.BLACK)
            contentDescription = getString(R.string.chatgpt_web_title)
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_YES
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
            ChatGptWebAuthenticationSupport.configure(settings)
            webViewClient = ChatGptWebViewClient(
                onPageStarted = {
                    this@ChatGptWebOfficialActivity.progress.visibility = View.VISIBLE
                },
                onPageReady = { url ->
                    cookieManager.flush()
                    sessionRestorer.onPageReady(url)
                },
                onBlockedNavigation = { host ->
                    Toast.makeText(
                        this@ChatGptWebOfficialActivity,
                        getString(R.string.chatgpt_web_blocked_host, host),
                        Toast.LENGTH_LONG,
                    ).show()
                },
                onPageError = { message -> showPageError(message) },
                rewriteAllowedMainFrameUrl = { null },
            )
            webChromeClient = object : WebChromeClient() {
                override fun onProgressChanged(view: WebView, newProgress: Int) {
                    this@ChatGptWebOfficialActivity.progress.progress = newProgress
                    this@ChatGptWebOfficialActivity.progress.visibility =
                        if (newProgress < 100) View.VISIBLE else View.GONE
                }

                override fun onShowFileChooser(
                    webView: WebView,
                    filePathCallback: ValueCallback<Array<Uri>>,
                    fileChooserParams: FileChooserParams,
                ): Boolean = fileChooserController.show(
                    webView,
                    filePathCallback,
                    fileChooserParams,
                )

                override fun onPermissionRequest(request: PermissionRequest) {
                    runOnUiThread { audioPermissionController.handle(request) }
                }

                override fun onPermissionRequestCanceled(request: PermissionRequest) {
                    runOnUiThread { audioPermissionController.cancel(request) }
                }
            }
        }
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(webView, true)

        progress = ProgressBar(
            this,
            null,
            android.R.attr.progressBarStyleHorizontal,
        ).apply {
            max = 100
            isIndeterminate = false
        }
        setContentView(FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(
                webView,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
            addView(
                progress,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    dp(3),
                    Gravity.TOP,
                ),
            )
        })

        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (webView.canGoBack()) webView.goBack() else finish()
                }
            },
        )
        proxyController.prepare { status ->
            if (isFinishing || isDestroyed) return@prepare
            status.error?.let(::showPageError)
            if (savedInstanceState == null || webView.restoreState(savedInstanceState) == null) {
                webView.loadUrl(
                    ChatGptWebOfficialFallbackIntent.startUrl(intent)
                        ?: sessionRestorer.restoreUrl(),
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        webView.onResume()
    }

    override fun onPause() {
        cookieManager.flush()
        webView.onPause()
        super.onPause()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        webView.saveState(outState)
        super.onSaveInstanceState(outState)
    }

    override fun onDestroy() {
        fileChooserController.dispose()
        audioPermissionController.dispose()
        webView.apply {
            stopLoading()
            webChromeClient = null
            destroy()
        }
        super.onDestroy()
    }

    private fun showMicrophoneDenied() {
        Toast.makeText(
            this,
            R.string.chatgpt_native_microphone_denied,
            Toast.LENGTH_LONG,
        ).show()
    }

    private fun showPageError(message: String) {
        progress.visibility = View.GONE
        Toast.makeText(this, message.take(160), Toast.LENGTH_LONG).show()
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}

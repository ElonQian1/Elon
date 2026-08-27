package com.elon.app.googleweb

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.webkit.CookieManager
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

class GoogleWebOfficialActivity : AppCompatActivity() {
    private var webView: WebView? = null

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.BLACK)
        }
        val view = WebView(this).apply {
            setBackgroundColor(Color.BLACK)
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                allowFileAccess = false
                allowContentAccess = false
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
            }
            webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    if (!request.isForMainFrame) return false
                    val url = request.url.toString()
                    if (GoogleWebNavigationPolicy.allows(url)) return false
                    openSystemBrowser(request.url)
                    return true
                }
            }
        }
        CookieManager.getInstance().apply {
            setAcceptCookie(true)
            setAcceptThirdPartyCookies(view, true)
        }
        root.addView(toolbar(view), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(56),
        ))
        root.addView(view, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1f,
        ))
        setContentView(root)
        webView = view
        val requestedUrl = intent.getStringExtra(EXTRA_START_URL)
        view.loadUrl(
            GoogleWebNavigationPolicy.sanitizeNavigableUrl(requestedUrl)
                ?: GoogleWebNavigationPolicy.START_URL,
        )
        onBackPressedDispatcher.addCallback(this, object : androidx.activity.OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (view.canGoBack()) view.goBack() else finish()
            }
        })
    }

    override fun onDestroy() {
        CookieManager.getInstance().flush()
        webView?.destroy()
        webView = null
        super.onDestroy()
    }

    private fun toolbar(view: WebView) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(6), 0, dp(6), 0)
        setBackgroundColor(Color.parseColor("#111111"))
        addView(iconButton(android.R.drawable.ic_media_previous, "返回") {
            if (view.canGoBack()) view.goBack() else finish()
        })
        addView(TextView(this@GoogleWebOfficialActivity).apply {
            text = "Google 搜索网页 AI"
            textSize = 18f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER_VERTICAL
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        addView(iconButton(android.R.drawable.ic_popup_sync, "刷新") { view.reload() })
    }

    private fun iconButton(resource: Int, description: String, action: () -> Unit) = ImageButton(this).apply {
        layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
        setImageResource(resource)
        setBackgroundColor(Color.TRANSPARENT)
        contentDescription = description
        setColorFilter(Color.WHITE)
        setOnClickListener { action() }
    }

    private fun openSystemBrowser(uri: Uri) {
        runCatching { startActivity(Intent(Intent.ACTION_VIEW, uri)) }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    companion object {
        private const val EXTRA_START_URL = "google_web_start_url"

        fun createIntent(context: Context, startUrl: String? = null) =
            Intent(context, GoogleWebOfficialActivity::class.java).apply {
                GoogleWebNavigationPolicy.sanitizeNavigableUrl(startUrl)?.let { safeUrl ->
                    putExtra(EXTRA_START_URL, safeUrl)
                }
            }
    }
}

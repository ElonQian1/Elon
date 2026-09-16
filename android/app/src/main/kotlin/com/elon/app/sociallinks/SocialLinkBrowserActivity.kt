package com.elon.app.sociallinks

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity

/** External content never receives the main chat WebView's JavaScript bridge or auth headers. */
class SocialLinkBrowserActivity : AppCompatActivity() {
    private lateinit var web: WebView
    private lateinit var status: TextView
    private lateinit var main: LinearLayout
    private lateinit var root: FrameLayout
    private var full: View? = null
    private var fullCallback: WebChromeClient.CustomViewCallback? = null
    private var original = ""
    private val handler = Handler(Looper.getMainLooper())
    private var readCapture: Runnable? = null
    private val timeout = Runnable { status.text = "若内容未显示，请刷新或打开原文。" }
    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        original = SocialLinkPolicy.safeUrl(intent.getStringExtra("url"))?.toString().orEmpty()
        if (original.isEmpty()) { finish(); return }
        val owner = com.elon.app.AuthManager.userId(applicationContext)
        val server = com.elon.app.ServerUrlManager.getActive(applicationContext)
        fun returnToChat() {
            var returned = false
            fun done() { if (!returned) { returned = true; finish() } }
            SocialLinkReadPreview.capture(web, original, server, owner) { done() }
            handler.postDelayed({ done() }, 800)
        }
        root = FrameLayout(this).apply { setBackgroundColor(Color.parseColor("#15171B")) }
        main = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        fun dp(n: Int) = (resources.displayMetrics.density * n).toInt()
        val heading = TextView(this).apply { text = intent.getStringExtra("title").orEmpty().ifBlank { Uri.parse(original).host }; textSize = 16f; setTextColor(Color.parseColor("#F1F2F5")); maxLines = 2; setPadding(dp(16), dp(10), dp(16), dp(6)) }
        val controls = LinearLayout(this)
        fun button(label: String, action: () -> Unit) {
            controls.addView(TextView(this).apply { text = label; textSize = 14f; setTextColor(Color.parseColor("#C5D6EC")); gravity = android.view.Gravity.CENTER; minHeight = dp(48); isFocusable = true; setOnClickListener { action() } }, LinearLayout.LayoutParams(0, -2, 1f))
        }
        button("返回聊天") { returnToChat() }; button("后退") { if (web.canGoBack()) web.goBack() }
        button("前进") { if (web.canGoForward()) web.goForward() }; button("刷新") { web.reload() }; button("打开原文") { external(original) }
        val xId = intent.getStringExtra("x_id").orEmpty()
        if (xId.matches(Regex("[0-9]{5,24}"))) button("嵌入查看") { embeddedX(xId) }
        status = TextView(this).apply { textSize = 12f; setTextColor(Color.parseColor("#B7BDC8")); setPadding(dp(16), dp(4), dp(16), dp(8)); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        web = WebView(this).apply {
            setBackgroundColor(Color.parseColor("#15171B"))
            settings.javaScriptEnabled = true; settings.domStorageEnabled = true
            settings.allowFileAccess = false; settings.allowContentAccess = false
            settings.mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            settings.mediaPlaybackRequiresUserGesture = true
            settings.setSupportMultipleWindows(false)
        }
        web.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (SocialLinkPolicy.safeUrl(request.url.toString()) != null) return false
                if (request.isForMainFrame) status.text = "该跳转需要原平台应用，可使用“打开原文”。"
                return true
            }
            override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) { readCapture?.let { handler.removeCallbacks(it) }; handler.removeCallbacks(timeout); status.text = "正在打开…"; handler.postDelayed(timeout, 10000) }
            override fun onPageFinished(view: WebView, url: String?) {
                handler.removeCallbacks(timeout); status.text = "内容由原平台提供；无法加载或需要登录时，可打开原文。"
                readCapture?.let { handler.removeCallbacks(it) }
                var attempts = 0
                readCapture = object : Runnable {
                    override fun run() {
                        if (isFinishing || isDestroyed) return
                        SocialLinkReadPreview.capture(view, original, server, owner)
                        if (++attempts < 12) handler.postDelayed(this, 1500)
                    }
                }.also { handler.post(it) }
            }
            override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
                if (request.isForMainFrame) { handler.removeCallbacks(timeout); status.text = "页面加载失败，请刷新或打开原文。" }
            }
        }
        web.webChromeClient = object : WebChromeClient() {
            override fun onShowCustomView(view: View, callback: CustomViewCallback) {
                if (full != null) { callback.onCustomViewHidden(); return }
                full = view; fullCallback = callback; main.visibility = View.GONE; root.addView(view, FrameLayout.LayoutParams(-1, -1))
            }
            override fun onHideCustomView() { hideFullscreen() }
        }
        main.addView(heading); main.addView(controls); main.addView(status); main.addView(web, LinearLayout.LayoutParams(-1, 0, 1f)); root.addView(main, FrameLayout.LayoutParams(-1, -1)); setContentView(root)
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val bars = insets.getInsets(androidx.core.view.WindowInsetsCompat.Type.systemBars())
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom); insets
        }
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() { if (full != null) hideFullscreen() else if (web.canGoBack()) web.goBack() else returnToChat() }
        })
        if (savedInstanceState != null && web.restoreState(savedInstanceState) != null) return
        val x = intent.getStringExtra("x_id").orEmpty()
        if (x.matches(Regex("[0-9]{5,24}")) && SocialLinkReadIdentity.identity(original) == null) {
            embeddedX(x)
        } else {
            val player = SocialLinkPolicy.safeUrl(intent.getStringExtra("player"))?.toString()
            web.loadUrl(player ?: SocialLinkReadIdentity.readingUrl(original))
        }
    }
    private fun embeddedX(id: String) {
        if (!id.matches(Regex("[0-9]{5,24}"))) return
        web.loadDataWithBaseURL("https://social-link.invalid/", """<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><style>body{background:#15171b;color:#eee}a{color:#bdd4ef}</style><blockquote class="twitter-tweet" data-theme="dark" data-dnt="true"><a href="https://x.com/i/status/$id">在 X 查看原帖</a></blockquote><script async src="https://platform.x.com/widgets.js"></script>""", "text/html", "UTF-8", null)
    }
    private fun external(url: String) { runCatching { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }.onFailure { Toast.makeText(this, "未找到可打开网页的浏览器", Toast.LENGTH_SHORT).show() } }
    private fun hideFullscreen() { full?.let { root.removeView(it) }; full = null; fullCallback?.onCustomViewHidden(); fullCallback = null; main.visibility = View.VISIBLE }
    override fun onSaveInstanceState(outState: Bundle) { if (::web.isInitialized) web.saveState(outState); super.onSaveInstanceState(outState) }
    override fun onPause() { if (::web.isInitialized) web.onPause(); super.onPause() }
    override fun onResume() { super.onResume(); if (::web.isInitialized) web.onResume() }
    override fun onDestroy() { handler.removeCallbacksAndMessages(null); if (::web.isInitialized) { hideFullscreen(); web.stopLoading(); main.removeView(web); web.destroy() }; super.onDestroy() }
    companion object {
        internal fun open(context: Context, item: SocialLink) {
            context.startActivity(Intent(context, SocialLinkBrowserActivity::class.java).putExtra("url", item.url).putExtra("title", item.title.ifBlank { item.site }).putExtra("player", item.player).putExtra("x_id", item.xId))
        }
    }
}

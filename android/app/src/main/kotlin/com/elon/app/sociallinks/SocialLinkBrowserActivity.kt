package com.elon.app.sociallinks

import android.app.PictureInPictureParams
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.util.Rational
import android.view.Gravity
import android.view.View
import android.webkit.WebChromeClient
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import android.widget.ProgressBar
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.Lifecycle
import com.elon.app.R

/**
 * Thin host for a [SocialLinkReaderSessions.Session]. External content never receives the main
 * chat WebView's JavaScript bridge or auth headers. Leaving (back / 返回聊天) minimises the session
 * into the chat's floating bubble; only 关闭 destroys it. Fullscreen video can continue in
 * picture-in-picture, and chat messages arriving meanwhile surface in an in-page banner.
 */
class SocialLinkBrowserActivity : AppCompatActivity(), SocialLinkReaderSessions.Ui {
    private lateinit var status: TextView
    private lateinit var progress: ProgressBar
    private lateinit var main: LinearLayout
    private lateinit var root: FrameLayout
    private lateinit var webHost: FrameLayout
    private lateinit var inbox: SocialLinkReaderInboxBar
    private lateinit var pipButton: TextView
    private var session: SocialLinkReaderSessions.Session? = null
    private var full: View? = null
    private var closing = false
    private val pipSupported by lazy { packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val link = SocialLinkPolicy.link(intent.getStringExtra("url").orEmpty(), intent.getStringExtra("title").orEmpty())
        if (link == null) { finish(); return }
        root = FrameLayout(this).apply { setBackgroundColor(Color.parseColor("#15171B")) }
        main = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        fun dp(n: Int) = (resources.displayMetrics.density * n).toInt()
        val heading = TextView(this).apply { text = link.title.ifBlank { Uri.parse(link.url).host }; textSize = 16f; setTextColor(Color.parseColor("#F1F2F5")); maxLines = 2; setPadding(dp(16), dp(10), dp(16), dp(6)) }
        val controls = LinearLayout(this)
        fun button(label: String, semanticId: Int, action: () -> Unit) {
            controls.addView(TextView(this).apply { text = label; id = semanticId; textSize = 14f; setTextColor(Color.parseColor("#C5D6EC")); gravity = android.view.Gravity.CENTER; minHeight = dp(48); isFocusable = true; setOnClickListener { action() } }, LinearLayout.LayoutParams(0, -2, 1f))
        }
        button("返回聊天", R.id.external_reader_chat) { minimize() }
        button("后退", R.id.external_reader_back) { session?.web?.let { if (it.canGoBack()) it.goBack() } }
        button("刷新", R.id.external_reader_refresh) { session?.web?.reload() }
        button("打开原文", R.id.external_reader_original) { external(link.url) }
        link.xId?.let { id -> button("嵌入查看", R.id.external_reader_embed) { embeddedX(id) } }
        button("关闭", R.id.external_reader_close) { close() }
        status = TextView(this).apply { textSize = 12f; setTextColor(Color.parseColor("#B7BDC8")); setPadding(dp(16), dp(4), dp(16), dp(8)); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        progress = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply { isIndeterminate = true; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }
        inbox = SocialLinkReaderInboxBar(this) { minimize() }
        webHost = FrameLayout(this)
        main.addView(heading); main.addView(controls); main.addView(status); main.addView(progress, LinearLayout.LayoutParams(-1, dp(2))); main.addView(inbox); main.addView(webHost, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(main, FrameLayout.LayoutParams(-1, -1))
        pipButton = TextView(this).apply {
            text = "画中画"; textSize = 13f; setTextColor(Color.WHITE); setBackgroundColor(Color.parseColor("#99000000")); gravity = Gravity.CENTER
            minHeight = dp(40); setPadding(dp(14), 0, dp(14), 0); visibility = View.GONE; setOnClickListener { enterPip() }
        }
        root.addView(pipButton, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.END).apply { topMargin = dp(12); rightMargin = dp(12) })
        setContentView(root)
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val bars = insets.getInsets(androidx.core.view.WindowInsetsCompat.Type.systemBars())
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom); insets
        }
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                val current = session; val web = current?.web
                if (full != null && current != null) SocialLinkReaderSessions.exitFullscreen(current) else if (web != null && web.canGoBack()) web.goBack() else minimize()
            }
        })
        val fresh = SocialLinkReaderSessions.all().none { it.key == SocialLinkReaderSessions.key(link) }
        val current = SocialLinkReaderSessions.obtain(this, link)
        session = current
        SocialLinkReaderSessions.attach(current, this, webHost, this)
        if (fresh) {
            if (link.xId != null && SocialLinkReadIdentity.identity(link.url) == null) embeddedX(link.xId) else current.web.loadUrl(current.key)
        }
    }

    private fun minimize() {
        val current = session ?: run { finish(); return }
        if (closing) return
        SocialLinkReaderInbox.clear()
        SocialLinkReaderSessions.detach(current, applicationContext)
        session = null
        if (!isInPictureInPictureMode) Toast.makeText(this, "已缩小为浮窗，可继续聊天", Toast.LENGTH_SHORT).show()
        finish()
    }

    private fun close() {
        closing = true
        SocialLinkReaderInbox.clear()
        session?.let { SocialLinkReaderSessions.destroy(it) }
        session = null
        finish()
    }

    override fun onStatus(text: String) { if (status.text.toString() != text) status.text = text }
    override fun onLoading(loading: Boolean) { progress.visibility = if (loading) View.VISIBLE else View.INVISIBLE }
    override fun onShowFullscreen(view: View, callback: WebChromeClient.CustomViewCallback) {
        full = view; main.visibility = View.GONE; root.addView(view, 0, FrameLayout.LayoutParams(-1, -1))
        pipButton.visibility = if (pipSupported && !isInPictureInPictureMode) View.VISIBLE else View.GONE
    }
    override fun onHideFullscreen() {
        full?.let { root.removeView(it) }; full = null; main.visibility = View.VISIBLE; pipButton.visibility = View.GONE
    }

    // Picture-in-picture is offered only for the page's own fullscreen video, never for the chrome.
    private fun enterPip() {
        if (full == null || !pipSupported || isInPictureInPictureMode) return
        runCatching { enterPictureInPictureMode(PictureInPictureParams.Builder().setAspectRatio(Rational(16, 9)).build()) }
            .onFailure { Toast.makeText(this, "当前系统不允许画中画", Toast.LENGTH_SHORT).show() }
    }
    override fun onUserLeaveHint() { super.onUserLeaveHint(); if (full != null) enterPip() }
    override fun onPictureInPictureModeChanged(isInPictureInPictureMode: Boolean, newConfig: Configuration) {
        super.onPictureInPictureModeChanged(isInPictureInPictureMode, newConfig)
        pipButton.visibility = if (full != null && pipSupported && !isInPictureInPictureMode) View.VISIBLE else View.GONE
        // Dismissing the PiP window leaves the Activity stopped; keep the page alive in the bubble.
        if (!isInPictureInPictureMode && lifecycle.currentState == Lifecycle.State.CREATED) minimize()
    }

    private fun embeddedX(id: String) {
        if (!id.matches(Regex("[0-9]{5,24}"))) return
        session?.web?.loadDataWithBaseURL("https://social-link.invalid/", """<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><style>body{background:#15171b;color:#eee}a{color:#bdd4ef}</style><blockquote class="twitter-tweet" data-theme="dark" data-dnt="true"><a href="https://x.com/i/status/$id">在 X 查看原帖</a></blockquote><script async src="https://platform.x.com/widgets.js"></script>""", "text/html", "UTF-8", null)
    }
    private fun external(url: String) { runCatching { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }.onFailure { Toast.makeText(this, "未找到可打开网页的浏览器", Toast.LENGTH_SHORT).show() } }

    override fun onStart() { super.onStart(); inbox.attach() }
    override fun onStop() { inbox.detach(); super.onStop() }
    // In picture-in-picture the Activity is paused but visible; the video must keep playing.
    override fun onPause() { if (!isInPictureInPictureMode) session?.web?.onPause(); super.onPause() }
    override fun onResume() { super.onResume(); session?.let { it.web.onResume(); it.diagnostics.resume() } }
    override fun onDestroy() {
        // System-initiated destruction (not 返回聊天/关闭) also keeps the page alive in the bubble.
        session?.let { if (!closing) SocialLinkReaderSessions.detach(it, applicationContext) }
        session = null
        super.onDestroy()
    }

    companion object {
        internal fun open(context: Context, item: SocialLink) {
            context.startActivity(Intent(context, SocialLinkBrowserActivity::class.java).putExtra("url", item.url).putExtra("title", item.title.ifBlank { item.site }).putExtra("player", item.player).putExtra("x_id", item.xId))
        }
    }
}

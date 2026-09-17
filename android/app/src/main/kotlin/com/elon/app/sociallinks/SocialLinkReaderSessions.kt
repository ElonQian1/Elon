package com.elon.app.sociallinks

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Context
import android.content.MutableContextWrapper
import android.graphics.Bitmap
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager

/**
 * Reading sessions outlive the reader Activity: minimising keeps the WebView alive (WeChat 浮窗
 * style) so the article keeps loading, read-back keeps running and returning is instant.
 * The WebView is created on a [MutableContextWrapper] so it can follow the Activity that shows it.
 */
internal object SocialLinkReaderSessions {
    const val MAX_SESSIONS = 3

    interface Ui {
        fun onStatus(text: String)
        fun onLoading(loading: Boolean)
        fun onShowFullscreen(view: View, callback: WebChromeClient.CustomViewCallback)
        fun onHideFullscreen()
    }

    class Session internal constructor(val link: SocialLink, val key: String, internal val context: MutableContextWrapper) {
        lateinit var web: WebView; internal set
        var ui: Ui? = null; internal set
        var loading: Boolean = true; internal set
        var minimized: Boolean = false; internal set
        var title: String = link.title.ifBlank { link.site }; internal set
        var lastUsed: Long = tick(); internal set
        internal var readAttempts = 0
        internal var fullscreenCallback: WebChromeClient.CustomViewCallback? = null
    }

    private val handler = Handler(Looper.getMainLooper())
    private val sessions = LinkedHashMap<String, Session>()
    private val listeners = mutableListOf<() -> Unit>()
    private var clock = 0L
    // Recency must be strictly ordered; several sessions can be touched within one millisecond.
    private fun tick(): Long = synchronized(this) { ++clock }

    fun key(link: SocialLink): String = SocialLinkPolicy.safeUrl(link.player)?.toString() ?: SocialLinkReadIdentity.readingUrl(link.url)
    @Synchronized fun all(): List<Session> = sessions.values.sortedByDescending { it.lastUsed }
    @Synchronized fun minimized(): List<Session> = all().filter { it.minimized }
    fun addListener(listener: () -> Unit) { synchronized(this) { listeners += listener } }
    fun removeListener(listener: () -> Unit) { synchronized(this) { listeners -= listener } }
    private fun notifyChanged() { val copy = synchronized(this) { listeners.toList() }; handler.post { copy.forEach { it() } } }

    /** Returns the live session for [link], creating it (and evicting the oldest minimised one) if needed. */
    @SuppressLint("SetJavaScriptEnabled")
    @Synchronized fun obtain(activity: Activity, link: SocialLink): Session {
        val key = key(link)
        sessions[key]?.let { it.lastUsed = tick(); it.context.baseContext = activity; return it }
        while (sessions.size >= MAX_SESSIONS) {
            val victim = sessions.values.filter { it.minimized }.minByOrNull { it.lastUsed } ?: sessions.values.minByOrNull { it.lastUsed } ?: break
            destroy(victim)
        }
        val context = MutableContextWrapper(activity)
        val session = Session(link, key, context)
        session.web = WebView(context).apply {
            setBackgroundColor(Color.parseColor("#15171B"))
            settings.javaScriptEnabled = true; settings.domStorageEnabled = true
            settings.allowFileAccess = false; settings.allowContentAccess = false
            settings.mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            settings.mediaPlaybackRequiresUserGesture = true
            settings.setSupportMultipleWindows(false)
            webViewClient = client(session, activity.applicationContext)
            webChromeClient = chromeClient(session)
        }
        sessions[key] = session
        notifyChanged()
        return session
    }

    private fun client(session: Session, app: Context) = object : WebViewClient() {
        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
            if (SocialLinkPolicy.safeUrl(request.url.toString()) != null) return false
            if (request.isForMainFrame) session.ui?.onStatus("该跳转需要原平台应用，可使用“打开原文”。")
            return true
        }
        override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) {
            session.loading = true; session.readAttempts = 0
            session.ui?.onLoading(true); session.ui?.onStatus("正在打开…"); notifyChanged()
        }
        override fun onPageFinished(view: WebView, url: String?) {
            session.loading = false
            session.ui?.onLoading(false); session.ui?.onStatus("内容由原平台提供；无法加载或需要登录时，可打开原文。")
            notifyChanged()
            scheduleReadBack(session, app)
        }
        override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
            if (request.isForMainFrame) { session.loading = false; session.ui?.onLoading(false); session.ui?.onStatus("页面加载失败，请刷新或打开原文。"); notifyChanged() }
        }
    }

    private fun chromeClient(session: Session) = object : WebChromeClient() {
        override fun onReceivedTitle(view: WebView?, title: String?) {
            val clean = title?.trim().orEmpty().take(120)
            if (clean.isNotEmpty() && !SocialLinkShareText.isGeneric(clean, session.link.site)) { session.title = clean; notifyChanged() }
        }
        override fun onShowCustomView(view: View, callback: CustomViewCallback) {
            val ui = session.ui
            if (ui == null || session.fullscreenCallback != null) { callback.onCustomViewHidden(); return }
            session.fullscreenCallback = callback; ui.onShowFullscreen(view, callback)
        }
        override fun onHideCustomView() { session.fullscreenCallback = null; session.ui?.onHideFullscreen() }
    }

    /** Leave fullscreen video from the host's back button; the page is told its view was hidden. */
    fun exitFullscreen(session: Session) {
        val callback = synchronized(this) { session.fullscreenCallback.also { session.fullscreenCallback = null } }
        callback?.onCustomViewHidden()
        session.ui?.onHideFullscreen()
    }

    // Read-back keeps observing the page after the reader is minimised; the Activity is not needed.
    private fun scheduleReadBack(session: Session, app: Context) {
        val owner = AuthManager.userId(app); val server = ServerUrlManager.getActive(app)
        val original = session.link.url
        val task = object : Runnable {
            override fun run() {
                if (!isLive(session)) return
                SocialLinkReadPreview.capture(session.web, original, server, owner)
                if (++session.readAttempts < 12) handler.postDelayed(this, 1500)
            }
        }
        handler.post(task)
    }

    fun attach(session: Session, activity: Activity, container: ViewGroup, ui: Ui) {
        synchronized(this) { session.context.baseContext = activity; session.ui = ui; session.minimized = false; session.lastUsed = tick() }
        (session.web.parent as? ViewGroup)?.removeView(session.web)
        container.addView(session.web, ViewGroup.LayoutParams(-1, -1))
        session.web.onResume()
        ui.onLoading(session.loading)
        notifyChanged()
    }

    /** Minimise: keep the page alive for the floating bubble, drop Activity references. */
    fun detach(session: Session, app: Context) {
        val callback = synchronized(this) {
            session.ui = null; session.minimized = true; session.lastUsed = tick()
            session.fullscreenCallback.also { session.fullscreenCallback = null }
        }
        callback?.onCustomViewHidden()
        (session.web.parent as? ViewGroup)?.removeView(session.web)
        // Timers/media pause; the pending navigation and read-back still complete.
        session.web.onPause()
        session.context.baseContext = app.applicationContext
        notifyChanged()
    }

    @Synchronized fun isLive(session: Session) = sessions[session.key] === session

    fun destroy(session: Session) {
        val removed = synchronized(this) { sessions.remove(session.key) === session }
        if (!removed) return
        session.ui = null
        (session.web.parent as? ViewGroup)?.removeView(session.web)
        session.web.stopLoading(); session.web.destroy()
        notifyChanged()
    }

    fun destroyAll() { all().forEach { destroy(it) } }
}

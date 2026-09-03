package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.CookieManager
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.BuildConfig
import com.elon.app.configureWebChatBackgroundSurface
import java.nio.charset.StandardCharsets
import org.json.JSONArray

internal class ChatGptWebImageGallerySync(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val store: ChatGptWebImageAssetStore,
    private val onStateChanged: (ChatGptWebImageGallerySnapshot) -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private var webView: WebView? = null
    private var injected = false
    private val timeout = Runnable { finish(ChatGptWebImageGallerySnapshot.STATE_FAILED, 0) }
    private val script by lazy(LazyThreadSafetyMode.NONE) {
        "window.__elonChatGptAdapterTargetVersion=${ChatGptWebPageAdapter.ADAPTER_VERSION};\n" +
            "window.__elonChatGptCachedImageHandles=${JSONArray(store.handles().toList())};\n" +
            listOf(IMAGE_ASSET_SCRIPT, GALLERY_SCRIPT).joinToString("\n") { asset ->
                activity.assets.open(asset).use { input ->
                    input.reader(StandardCharsets.UTF_8).readText()
                }
            }
    }

    @SuppressLint("SetJavaScriptEnabled")
    fun start(): Boolean {
        cancel()
        if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
            onStateChanged(ChatGptWebImageGallerySnapshot(
                ChatGptWebImageGallerySnapshot.STATE_FAILED,
                0,
            ))
            return false
        }
        WebView.setWebContentsDebuggingEnabled(BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED)
        val view = WebView(activity).apply {
            configureWebChatBackgroundSurface()
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                allowFileAccess = false
                allowContentAccess = false
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
                mediaPlaybackRequiresUserGesture = true
                loadsImagesAutomatically = true
            }
            webViewClient = ChatGptWebViewClient(
                onPageStarted = { injected = false },
                onPageReady = { url -> injectIfAllowed(this, url) },
                onBlockedNavigation = { finish(ChatGptWebImageGallerySnapshot.STATE_FAILED, 0) },
                onPageError = { finish(ChatGptWebImageGallerySnapshot.STATE_FAILED, 0) },
                rewriteAllowedMainFrameUrl = { null },
            )
            visibility = View.VISIBLE
            alpha = 0.01f
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
            isFocusable = false
            isClickable = false
        }
        WebViewCompat.addWebMessageListener(
            view,
            BRIDGE_OBJECT,
            setOf(ALLOWED_ORIGIN),
        ) { _, message, sourceOrigin, isMainFrame, _ ->
            if (!isMainFrame || sourceOrigin.toString() != ALLOWED_ORIGIN) return@addWebMessageListener
            val event = message.data
                ?.let { ChatGptWebProtocol.parse(it, ChatGptWebPageAdapter.ADAPTER_VERSION) }
                ?: return@addWebMessageListener
            when (event) {
                is ChatGptWebEvent.ImageAsset -> store.save(event.value) { }
                is ChatGptWebEvent.ImageGallerySnapshot -> {
                    activity.runOnUiThread {
                        onStateChanged(event.value)
                        if (event.value.state != ChatGptWebImageGallerySnapshot.STATE_LOADING) {
                            cleanup()
                        }
                    }
                }
                else -> Unit
            }
        }
        CookieManager.getInstance().apply {
            setAcceptCookie(true)
            setAcceptThirdPartyCookies(view, true)
        }
        host.addView(view, FrameLayout.LayoutParams(SYNC_VIEW_WIDTH, SYNC_VIEW_HEIGHT))
        webView = view
        onStateChanged(ChatGptWebImageGallerySnapshot(
            ChatGptWebImageGallerySnapshot.STATE_LOADING,
            store.entries().size,
        ))
        handler.postDelayed(timeout, SYNC_TIMEOUT_MS)
        view.loadUrl(IMAGES_URL)
        return true
    }

    fun cancel() = cleanup()

    private fun injectIfAllowed(view: WebView, url: String) {
        if (injected || !ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) return
        if (ChatGptWebNavigationPolicy.isAuthenticationPage(url)) {
            finish(ChatGptWebImageGallerySnapshot.STATE_FAILED, 0)
            return
        }
        injected = true
        view.evaluateJavascript(script, null)
    }

    private fun finish(state: String, count: Int) {
        onStateChanged(ChatGptWebImageGallerySnapshot(state, count))
        cleanup()
    }

    private fun cleanup() {
        handler.removeCallbacks(timeout)
        val view = webView ?: return
        webView = null
        runCatching { WebViewCompat.removeWebMessageListener(view, BRIDGE_OBJECT) }
        view.stopLoading()
        view.webViewClient = WebViewClient()
        host.removeView(view)
        view.destroy()
        injected = false
    }

    private companion object {
        const val IMAGES_URL = "https://chatgpt.com/images"
        const val ALLOWED_ORIGIN = "https://chatgpt.com"
        const val BRIDGE_OBJECT = "elonChatGptImageGalleryNative"
        const val IMAGE_ASSET_SCRIPT = "chatgpt_web_image_assets.js"
        const val GALLERY_SCRIPT = "chatgpt_web_image_gallery_sync.js"
        const val SYNC_TIMEOUT_MS = 35_000L
        const val SYNC_VIEW_WIDTH = 720
        const val SYNC_VIEW_HEIGHT = 1_280
    }
}

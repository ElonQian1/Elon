package com.elon.app.grid.host

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.webkit.CookieManager
import android.webkit.SslErrorHandler
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.net.URI

internal fun binanceHostNavigation(raw: String): Boolean = runCatching {
    val uri = URI(raw)
    uri.scheme == "https" && uri.rawUserInfo == null && uri.port in setOf(-1, 443) &&
        uri.host in setOf("www.binance.com", "accounts.binance.com")
}.getOrDefault(false)

@SuppressLint("SetJavaScriptEnabled")
internal fun createBinanceHostWebView(context: Context, runtime: BinanceHostRuntime): WebView {
    require(WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER) &&
        WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) { "WEBVIEW_UPDATE_REQUIRED" }
    return WebView(context).apply {
        isSaveEnabled = false
        settings.apply {
            javaScriptEnabled = true; domStorageEnabled = true
            allowFileAccess = false; allowContentAccess = false
            mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            javaScriptCanOpenWindowsAutomatically = false; setSupportMultipleWindows(false)
            safeBrowsingEnabled = true; setGeolocationEnabled(false)
        }
        CookieManager.getInstance().setAcceptCookie(true)
        CookieManager.getInstance().setAcceptThirdPartyCookies(this, false)
        WebViewCompat.addWebMessageListener(this, "ElonBinanceRead", setOf(BinanceHostRuntime.ORIGIN)) { _, message, origin, mainFrame, _ ->
            if (mainFrame && origin.toString().trimEnd('/') == BinanceHostRuntime.ORIGIN) {
                message.data?.takeIf { it.length <= 262144 }?.let(runtime::observed)
            }
        }
        WebViewCompat.addDocumentStartJavaScript(this,
            context.assets.open("binance_grid_read_adapter.js").bufferedReader().use { it.readText() }, setOf(BinanceHostRuntime.ORIGIN))
        webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean = !binanceHostNavigation(request.url.toString())
            override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) = runtime.pageStarted(url)
            // Bind once the document is visible; slow third-party resources must not hold buffered observations.
            override fun onPageCommitVisible(view: WebView, url: String) = runtime.pageReady(url, finished = false)
            override fun onPageFinished(view: WebView, url: String) { CookieManager.getInstance().flush(); runtime.pageReady(url) }
            override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: android.net.http.SslError) {
                handler.cancel(); runtime.fail("币安连接证书验证失败")
            }
            override fun onReceivedError(view: WebView, request: WebResourceRequest, error: android.webkit.WebResourceError) {
                if (request.isForMainFrame) runtime.fail("币安页面未能加载，请检查网络后重试")
            }
            override fun onRenderProcessGone(view: WebView, detail: android.webkit.RenderProcessGoneDetail): Boolean {
                runtime.invalidate("币安网页进程已退出，请返回量化应用重新连接")
                return true
            }
        }
    }
}

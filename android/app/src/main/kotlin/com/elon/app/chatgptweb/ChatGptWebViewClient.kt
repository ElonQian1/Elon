package com.elon.app.chatgptweb

import android.graphics.Bitmap
import android.net.http.SslError
import android.webkit.SslErrorHandler
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient

internal class ChatGptWebViewClient(
    private val onPageStarted: (String) -> Unit,
    private val onPageReady: (String) -> Unit,
    private val onBlockedNavigation: (String) -> Unit,
    private val onPageError: (String) -> Unit,
    private val rewriteAllowedMainFrameUrl: (String) -> String?,
) : WebViewClient() {
    override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
        if (!request.isForMainFrame) return false

        val url = request.url.toString()
        if (!ChatGptWebNavigationPolicy.allows(url)) {
            onBlockedNavigation(ChatGptWebNavigationPolicy.displayHost(url))
            return true
        }

        if (request.method.equals("GET", ignoreCase = true)) {
            val rewrittenUrl = rewriteAllowedMainFrameUrl(url)
            if (rewrittenUrl != null && rewrittenUrl != url) {
                if (!ChatGptWebNavigationPolicy.allows(rewrittenUrl)) {
                    onBlockedNavigation(ChatGptWebNavigationPolicy.displayHost(rewrittenUrl))
                    return true
                }
                view.loadUrl(rewrittenUrl)
                return true
            }
        }

        return false
    }

    override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
        onPageStarted(url)
    }

    override fun onPageFinished(view: WebView, url: String) {
        onPageReady(url)
    }

    override fun onReceivedError(
        view: WebView,
        request: WebResourceRequest,
        error: WebResourceError,
    ) {
        if (request.isForMainFrame) {
            onPageError(error.description.toString().ifBlank { "页面加载失败" })
        }
    }

    override fun onReceivedHttpError(
        view: WebView,
        request: WebResourceRequest,
        errorResponse: WebResourceResponse,
    ) {
        if (request.isForMainFrame) {
            onPageError("页面返回 HTTP ${errorResponse.statusCode}")
        }
    }

    override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
        handler.cancel()
        onPageError("页面证书校验失败")
    }
}

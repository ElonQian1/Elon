package com.elon.app.sharing

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.articles.ArticleUi

/** External content has no native JS bridge, file access or permission grants. */
class ArticleLinkActivity : AppCompatActivity() {
    private var browser: WebView? = null
    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val url = SourceLink.webUrl(intent.getStringExtra("url")) ?: return finish()
        val ui = ArticleUi(this); val page = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val origin = ui.text(Uri.parse(url).host.orEmpty(), 13f, true)
        val actions = ui.row()
        actions.addView(ui.button("返回") { finish() })
        actions.addView(ui.button("发到聊天") { ExternalShareActivity.shareText(this, SourceLink.webUrl(browser?.url) ?: url) })
        actions.addView(ui.button("浏览器") { runCatching { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(SourceLink.webUrl(browser?.url) ?: url))) }.onFailure { Toast.makeText(this, "未找到浏览器", Toast.LENGTH_SHORT).show() } })
        for (index in 0 until actions.childCount) actions.getChildAt(index).layoutParams = LinearLayout.LayoutParams(0, -2, 1f)
        page.addView(actions); page.addView(origin)
        val web = WebView(this).also { browser = it }
        web.settings.apply {
            javaScriptEnabled = true; domStorageEnabled = true
            allowFileAccess = false; allowContentAccess = false
            mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            javaScriptCanOpenWindowsAutomatically = false
            setSupportMultipleWindows(false)
        }
        web.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean = SourceLink.webUrl(request.url.toString()) == null
            override fun onPageFinished(view: WebView, loaded: String) { origin.text = Uri.parse(loaded).host.orEmpty() }
            override fun onReceivedSslError(view: WebView, handler: android.webkit.SslErrorHandler, error: android.net.http.SslError) { handler.cancel(); origin.text = "连接证书异常，请使用浏览器核对" }
            override fun onReceivedError(view: WebView, request: WebResourceRequest, error: android.webkit.WebResourceError) {
                if (request.isForMainFrame) origin.text = "页面暂时无法加载，可用浏览器打开"
            }
        }
        page.addView(web, LinearLayout.LayoutParams(-1, 0, 1f)); installShareWindow(page)
        if (savedInstanceState == null || web.restoreState(savedInstanceState) == null) web.loadUrl(url)
    }
    override fun onSaveInstanceState(outState: Bundle) { browser?.saveState(outState); super.onSaveInstanceState(outState) }
    override fun onDestroy() { browser?.stopLoading(); browser?.destroy(); browser = null; super.onDestroy() }
    companion object {
        fun open(context: Context, url: String) {
            if (SourceLink.webUrl(url) != null) context.startActivity(Intent(context, ArticleLinkActivity::class.java).putExtra("url", url).addFlags(if (context is android.app.Activity) 0 else Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }
}

package com.elon.app.chatkit

import android.annotation.SuppressLint
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.MenuItem
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.AiProviderAccountsApi
import com.elon.app.AuthManager
import com.elon.app.ElonApplication
import com.elon.app.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class OpenAiChatKitActivity : AppCompatActivity() {
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(35, TimeUnit.SECONDS)
        .build()
    private val api by lazy { AiProviderAccountsApi(this, http) }

    private lateinit var status: TextView
    private lateinit var progress: ProgressBar
    private lateinit var retry: Button
    private lateinit var webView: WebView
    private var pageLoaded = false

    @SuppressLint("SetJavaScriptEnabled", "AddJavascriptInterface")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_openai_chatkit)
        supportActionBar?.apply {
            title = "OpenAI ChatKit"
            setDisplayHomeAsUpEnabled(true)
        }
        if (!AuthManager.isLoggedIn(this)) {
            Toast.makeText(this, "请先登录一龙账号", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        status = findViewById(R.id.openAiChatKitStatus)
        progress = findViewById(R.id.openAiChatKitProgress)
        retry = findViewById(R.id.openAiChatKitRetry)
        webView = findViewById(R.id.openAiChatKitWebView)
        retry.setOnClickListener { loadCapability() }

        webView.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = false
            mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
        }
        webView.addJavascriptInterface(ChatKitBridge(), BRIDGE_NAME)
        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (!request.isForMainFrame) return false
                val uri = request.url
                if (uri.scheme == "https") openExternal(uri)
                return true
            }
        }
        loadCapability()
    }

    private fun loadCapability() {
        progress.visibility = View.VISIBLE
        retry.isEnabled = false
        status.text = "正在检查 ChatKit 配置…"
        scope.launch {
            runCatching { withContext(Dispatchers.IO) { api.fetchChatKitCapability() } }
                .onSuccess { capability ->
                    if (capability.configured) {
                        status.text = "已使用当前一龙账号连接官方 ChatKit API"
                        if (!pageLoaded) loadChatKitPage()
                    } else {
                        webView.visibility = View.GONE
                        status.text = capability.message
                    }
                }
                .onFailure {
                    webView.visibility = View.GONE
                    status.text = "ChatKit 配置读取失败：${it.message.orEmpty().take(220)}"
                }
            progress.visibility = View.GONE
            retry.isEnabled = true
        }
    }

    private fun loadChatKitPage() {
        pageLoaded = true
        webView.visibility = View.VISIBLE
        val baseUrl = ElonApplication.activeServerUrl(this).trimEnd('/') + "/"
        webView.loadDataWithBaseURL(baseUrl, CHATKIT_HTML, "text/html", "UTF-8", null)
    }

    private inner class ChatKitBridge {
        @JavascriptInterface
        fun requestClientSecret() {
            scope.launch {
                runCatching { withContext(Dispatchers.IO) { api.createChatKitSession() } }
                    .onSuccess { secret ->
                        val quoted = JSONObject.quote(secret)
                        webView.evaluateJavascript("window.__elonResolveChatKitSecret($quoted)", null)
                    }
                    .onFailure { error ->
                        val quoted = JSONObject.quote(error.message.orEmpty().take(220))
                        webView.evaluateJavascript("window.__elonRejectChatKitSecret($quoted)", null)
                    }
            }
        }

        @JavascriptInterface
        fun reportStatus(message: String) {
            scope.launch { status.text = message.take(220) }
        }
    }

    private fun openExternal(uri: Uri) {
        runCatching { startActivity(Intent(Intent.ACTION_VIEW, uri)) }
            .onFailure { Toast.makeText(this, "无法打开外部链接", Toast.LENGTH_SHORT).show() }
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == android.R.id.home) {
            finish()
            return true
        }
        return super.onOptionsItemSelected(item)
    }

    override fun onDestroy() {
        if (::webView.isInitialized) {
            webView.removeJavascriptInterface(BRIDGE_NAME)
            webView.stopLoading()
            webView.destroy()
        }
        scope.cancel()
        http.dispatcher.executorService.shutdown()
        super.onDestroy()
    }

    private companion object {
        const val BRIDGE_NAME = "ElonChatKitBridge"
        const val CHATKIT_HTML = """
<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1">
  <style>
    html,body,openai-chatkit{width:100%;height:100%;margin:0;display:block;background:#101114;color:#e6e6e6}
    #loading{padding:24px;font:14px sans-serif;color:#9ca3af}
  </style>
  <script src="https://cdn.platform.openai.com/deployments/chatkit/chatkit.js" async></script>
</head>
<body>
  <div id="loading">正在加载官方 ChatKit…</div>
  <openai-chatkit id="chatkit" hidden></openai-chatkit>
  <script>
    let pendingResolve = null;
    let pendingReject = null;
    window.__elonResolveChatKitSecret = function(secret) {
      if (pendingResolve) pendingResolve(secret);
      pendingResolve = null;
      pendingReject = null;
    };
    window.__elonRejectChatKitSecret = function(message) {
      if (pendingReject) pendingReject(new Error(message || 'ChatKit session failed'));
      pendingResolve = null;
      pendingReject = null;
    };
    function getClientSecret() {
      return new Promise(function(resolve, reject) {
        pendingResolve = resolve;
        pendingReject = reject;
        ElonChatKitBridge.requestClientSecret();
      });
    }
    customElements.whenDefined('openai-chatkit').then(function() {
      const element = document.getElementById('chatkit');
      element.setOptions({api:{getClientSecret:getClientSecret},theme:'dark'});
      document.getElementById('loading').remove();
      element.hidden = false;
      ElonChatKitBridge.reportStatus('ChatKit 已就绪 · 当前身份是一龙账号');
      element.addEventListener('chatkit.response.start', function() { ElonChatKitBridge.reportStatus('OpenAI 正在回复…'); });
      element.addEventListener('chatkit.response.end', function() { ElonChatKitBridge.reportStatus('回复完成'); });
      element.addEventListener('chatkit.error', function(event) {
        const message = event.detail && event.detail.error && event.detail.error.message;
        ElonChatKitBridge.reportStatus(message || 'ChatKit 会话发生错误');
      });
    }).catch(function() { ElonChatKitBridge.reportStatus('ChatKit 组件加载失败'); });
  </script>
</body>
</html>
"""
    }
}

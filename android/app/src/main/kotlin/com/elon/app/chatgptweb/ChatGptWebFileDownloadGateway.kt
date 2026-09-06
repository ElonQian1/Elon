package com.elon.app.chatgptweb

import android.app.DownloadManager
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.SystemClock
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatConversationFile
import org.json.JSONObject

internal class ChatGptWebFileDownloadGateway(
    context: Context,
    private val webView: WebView,
    private val document: () -> WebBridgeDocumentSession.Snapshot,
) {
    private val app = context.applicationContext
    private val leases = ChatGptWebFileDownloadLease()
    private var installed = false
    private var disposed = false

    fun install() {
        if (installed || !WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) return
        WebViewCompat.addWebMessageListener(webView, BRIDGE, setOf(ORIGIN)) { _, message, origin, mainFrame, reply ->
            if (disposed || !mainFrame || origin.toString().trimEnd('/') != ORIGIN) return@addWebMessageListener
            val body = message.data?.takeIf { it.length <= 20_000 } ?: return@addWebMessageListener
            val value = runCatching { JSONObject(body) }.getOrNull() ?: return@addWebMessageListener
            val id = value.optString("leaseId").takeIf { UUID.matches(it) } ?: return@addWebMessageListener
            val result = JSONObject().put("leaseId", id).put("state", "failed")
            val state = document()
            val url = ChatGptWebFileDownloadPolicy.signedUrl(value.optString("url"))
            if (state.adapterCurrent && value.optString("documentToken") == state.documentToken) {
                val lease = leases.consume(id, state.documentToken, state.pageGeneration,
                    webView.url.orEmpty(), SystemClock.elapsedRealtime())
                if (lease != null && value.optBoolean("cancel")) result.put("state", "cancelled")
                else if (lease != null && url != null && runCatching { enqueue(lease, url) }.getOrDefault(false)) {
                    result.put("state", "queued")
                }
            }
            // No signed URLs, credentials or server error bodies enter the generic command receipts.
            runCatching { reply.postMessage(result.toString()) }
        }
        installed = true
    }

    fun prepare(path: String, file: WebChatConversationFile): String? {
        val state = document()
        val href = webView.url ?: return null
        val uri = Uri.parse(href)
        if (!installed || disposed || !state.adapterCurrent || uri.scheme != "https" ||
            uri.host != "chatgpt.com" || uri.port != -1 || !ChatGptWebFileDownloadPolicy.HANDLE.matches(file.downloadHandle)) return null
        val lease = leases.begin(state.documentToken, state.pageGeneration, href,
            file.name, file.mediaType, SystemClock.elapsedRealtime()) ?: return null
        return JSONObject().put("version", 1).put("leaseId", lease.id)
            .put("documentToken", lease.token).put("href", href).put("path", path)
            .put("name", file.name).put("downloadHandle", file.downloadHandle).toString()
    }

    private fun enqueue(lease: ChatGptWebFileDownloadLease.Value, url: String): Boolean {
        val manager = app.getSystemService(Context.DOWNLOAD_SERVICE) as? DownloadManager ?: return false
        val request = DownloadManager.Request(Uri.parse(url))
            .setTitle(lease.name)
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
        if (lease.mediaType.isNotBlank()) request.setMimeType(lease.mediaType)
        val destination = "elon-${lease.id}-${lease.name}"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            request.setDestinationInExternalPublicDir(Environment.DIRECTORY_DOWNLOADS, destination)
        } else {
            // No broad storage permission is needed on the supported Android 8/9 path.
            request.setDestinationInExternalFilesDir(app, Environment.DIRECTORY_DOWNLOADS, destination)
        }
        // The URL is already authorized. Do not copy WebView Cookie or Authorization headers.
        return manager.enqueue(request) > 0L
    }

    fun cancel() { leases.cancel() }
    fun dispose() {
        cancel()
        disposed = true
        if (installed) WebViewCompat.removeWebMessageListener(webView, BRIDGE)
        installed = false
    }

    private companion object {
        const val BRIDGE = "elonChatGptFileDownload"
        const val ORIGIN = "https://chatgpt.com"
        val UUID = Regex("[a-f0-9-]{36}")
    }
}

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
import com.elon.app.WebChatFileDownloadState
import com.elon.app.WebChatFileDownloadState.Stage
import org.json.JSONObject

internal class ChatGptWebFileDownloadGateway(
    context: Context,
    private val webView: WebView,
    private val document: () -> WebBridgeDocumentSession.Snapshot,
) {
    private val app = context.applicationContext
    private val leases = ChatGptWebFileDownloadLease()
    private val session = ChatGptWebFileDownloadSession()
    private val bytes = ChatGptWebFileByteDownload(app, isCurrent = { lease ->
        val state = document()
        !disposed && state.adapterCurrent && state.documentToken == lease.token &&
            state.pageGeneration == lease.generation && webView.url == lease.href
    }, onProgress = session::update)
    private var installed = false
    private var disposed = false

    fun install() {
        if (installed || !WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) return
        WebViewCompat.addWebMessageListener(webView, BRIDGE, setOf(ORIGIN)) { _, message, origin, mainFrame, reply ->
            if (disposed || !mainFrame || origin.toString().trimEnd('/') != ORIGIN) return@addWebMessageListener
            val body = message.data?.takeIf { it.length <= 70_000 } ?: return@addWebMessageListener
            val value = runCatching { JSONObject(body) }.getOrNull() ?: return@addWebMessageListener
            val id = value.optString("leaseId").takeIf { UUID.matches(it) } ?: return@addWebMessageListener
            val result = JSONObject().put("leaseId", id).put("state", "failed")
            val state = document()
            if (value.has("byteOperation")) {
                if (!state.adapterCurrent || value.optString("documentToken") != state.documentToken) return@addWebMessageListener
                val lease = if (value.optString("byteOperation") == "begin") {
                    consumeResolved(id, value, state)
                } else null
                bytes.accept(value, lease) { response -> runCatching { reply.postMessage(response) } }
                return@addWebMessageListener
            }
            val url = ChatGptWebFileDownloadPolicy.signedUrl(value.optString("url"))
            if (state.adapterCurrent && value.optString("documentToken") == state.documentToken) {
                if (value.optBoolean("cancel") && !bytes.cancel(id)) session.pageCancelled(id)
                val lease = consumeResolved(id, value, state)
                if (lease != null && value.optBoolean("cancel")) result.put("state", "cancelled")
                else if (lease != null && url != null && runCatching { enqueue(lease, url) }.getOrDefault(false)) {
                    result.put("state", "queued")
                    session.update(id, Stage.QUEUED)
                } else if (lease != null) {
                    session.update(id, Stage.FAILED)
                }
            }
            // No signed URLs, credentials or server error bodies enter the generic command receipts.
            runCatching { reply.postMessage(result.toString()) }
        }
        installed = true
    }

    fun prepare(path: String, file: WebChatConversationFile, requestId: String): String? {
        val state = document()
        val href = webView.url ?: return null
        val uri = Uri.parse(href)
        if (!installed || disposed || session.snapshot()?.active == true || requestId.isBlank() || !state.adapterCurrent || uri.scheme != "https" ||
            uri.host != "chatgpt.com" || uri.port != -1 || !ChatGptWebFileDownloadPolicy.HANDLE.matches(file.downloadHandle)) return null
        val lease = leases.begin(state.documentToken, state.pageGeneration, href,
            file.name, file.mediaType, SystemClock.elapsedRealtime()) ?: return null
        check(session.begin(lease.id, requestId))
        return JSONObject().put("version", 1).put("leaseId", lease.id)
            .put("byteTransferVersion", 1)
            .put("resolvedFileVersion", ChatGptWebFileDownloadMetadata.VERSION)
            .put("documentToken", lease.token).put("href", href).put("path", path)
            .put("name", file.name).put("downloadHandle", file.downloadHandle).toString()
    }

    private fun consumeResolved(id: String, value: JSONObject, state: WebBridgeDocumentSession.Snapshot): ChatGptWebFileDownloadLease.Value? {
        val lease = leases.consume(id, state.documentToken, state.pageGeneration,
            webView.url.orEmpty(), SystemClock.elapsedRealtime()) ?: return null
        return ChatGptWebFileDownloadMetadata.resolve(lease, value).also {
            if (it == null) session.update(id, Stage.FAILED)
        }
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

    fun snapshot(): WebChatFileDownloadState? = session.snapshot()

    fun cancelDownload(requestId: String): Boolean {
        val current = session.snapshot()
        if (current?.requestId != requestId || !current.active) return false
        if (current.stage == Stage.CANCELLING) return true
        val id = session.requestCancel(requestId) ?: return false
        leases.cancel()
        if (!bytes.cancel(id)) session.update(id, Stage.CANCELLED)
        // This calls only our page-local owner, never an official DOM control.
        runCatching { webView.evaluateJavascript("window.__elonChatGptPrivateFileDownload?.cancel(" + JSONObject.quote(id) + ");", null) }
        return true
    }

    fun cancel() {
        session.snapshot()?.takeIf { it.active }?.let { cancelDownload(it.requestId) }
        leases.cancel()
        bytes.cancel()
    }
    fun dispose() {
        cancel()
        disposed = true
        bytes.dispose()
        if (installed) WebViewCompat.removeWebMessageListener(webView, BRIDGE)
        installed = false
    }

    private companion object {
        const val BRIDGE = "elonChatGptFileDownload"
        const val ORIGIN = "https://chatgpt.com"
        val UUID = Regex("[a-f0-9-]{36}")
    }
}

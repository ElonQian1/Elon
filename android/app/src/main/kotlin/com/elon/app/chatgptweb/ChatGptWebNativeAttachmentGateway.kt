package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Base64
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.PendingAttachment
import com.elon.app.WebBridgeDocumentSession
import java.util.UUID
import java.util.concurrent.Executors
import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebNativeAttachmentGateway(
    context: Context,
    private val webView: WebView,
    private val document: () -> WebBridgeDocumentSession.Snapshot,
) {
    private val resolver = context.applicationContext.contentResolver
    private val authority = "${context.packageName}.fileprovider"
    private val main = Handler(Looper.getMainLooper())
    private val ioDelegate = lazy { Executors.newSingleThreadExecutor() }
    private val io by ioDelegate
    private var installed = false
    private var disposed = false
    private val leases = linkedMapOf<String, Lease>()
    private var reading: Lease? = null
    val pickerPreparation = ChatGptWebAttachmentPickerPreparation(
        document, { webView.url }, { installed && !disposed },
        { script -> webView.evaluateJavascript(script, null) },
        { task, delay -> main.postDelayed(task, delay) }, main::removeCallbacks, SystemClock::elapsedRealtime,
    )

    private data class Lease(
        val id: String,
        val documentToken: String,
        val generation: Long,
        val href: String,
        val expiresAt: Long,
        val reader: ChatGptWebNativeAttachmentReader,
    )

    fun install() {
        if (installed || !WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) return
        WebViewCompat.addWebMessageListener(webView, BRIDGE, setOf(ORIGIN)) { _, message, origin, isMainFrame, reply ->
            if (disposed || !isMainFrame || origin.toString().trimEnd('/') != ORIGIN) return@addWebMessageListener
            val payload = message.data?.takeIf { it.length <= 4096 } ?: return@addWebMessageListener
            val request = runCatching { JSONObject(payload) }.getOrNull() ?: return@addWebMessageListener
            val id = request.optString("requestId").takeIf { REQUEST_ID.matches(it) } ?: return@addWebMessageListener
            val current = leases[request.optString("leaseId")]
            val offset = request.optInt("offset", -1)
            if (current == null || reading != null || !isCurrent(current) || request.optString("leaseId") != current.id ||
                request.optString("documentToken") != current.documentToken || offset < 0
            ) {
                reply.postMessage(JSONObject().put("requestId", id).put("code", "attachment_read_expired").toString())
                return@addWebMessageListener
            }
            reading = current
            io.execute {
                val bytes = runCatching { current.reader.read(offset) }.getOrNull()
                main.post {
                    if (reading === current) reading = null
                    val result = JSONObject().put("requestId", id)
                    if (bytes == null || !isCurrent(current)) {
                        result.put("code", "attachment_read_expired")
                    } else {
                        result.put("offset", offset).put("data", Base64.encodeToString(bytes, Base64.NO_WRAP))
                    }
                    // ReplyProxy remains tied to the originating frame, never a replacement document.
                    if (!disposed) runCatching { reply.postMessage(result.toString()) }
                }
            }
        }
        installed = true
    }

    fun prepare(attachments: List<PendingAttachment>, uris: List<Uri>): String? {
        if (!installed || disposed || attachments.size !in 1..ChatGptWebNativeAttachmentPolicy.MAX_FILES ||
            uris.size != attachments.size
        ) {
            pickerPreparation.cancel()
            return null
        }
        val state = document()
        val href = webView.url ?: return null
        if (!state.adapterCurrent || attachments.zip(uris).any { (file, uri) ->
                uri.scheme != "content" || uri.authority != authority ||
                    !ChatGptWebNativeAttachmentPolicy.supports(file.mimeType, file.file.length(), file.imageWidth, file.imageHeight)
            } ||
            Uri.parse(href).let { it.scheme != "https" || it.host != "chatgpt.com" || it.port != -1 }
        ) {
            pickerPreparation.cancel()
            return null
        }
        val selectionId = if (attachments.size != 1 || attachments.single().chatGptUploadCopy) {
            pickerPreparation.cancel()
            null
        } else pickerPreparation.take(attachments.single())
        revokeLease(preserveSelection = true)
        val expiresAt = SystemClock.elapsedRealtime() + 120_000L
        val descriptors = attachments.zip(uris).mapIndexed { index, (file, uri) ->
            val size = file.file.length().toInt()
            val next = Lease(
                UUID.randomUUID().toString(), state.documentToken, state.pageGeneration, href, expiresAt,
                ChatGptWebNativeAttachmentReader(size) { requireNotNull(resolver.openInputStream(uri)) },
            )
            leases[next.id] = next
            JSONObject().put("version", 1).put("leaseId", next.id)
                .put("uploadCopy", file.chatGptUploadCopy).put("selectionId", selectionId)
                .put("documentToken", next.documentToken).put("href", href)
                .put("name", ChatGptWebUploadPolicy.stagedName(file.displayName, file.fileName, index))
                .put("size", size).put("type", file.mimeType)
                .put("width", file.imageWidth).put("height", file.imageHeight)
        }
        val first = leases.values.first()
        main.postDelayed({ if (leases[first.id] === first) cancel() }, 120_000L)
        return if (descriptors.size == 1) descriptors.single().toString() else
            JSONObject().put("version", 2).put("documentToken", state.documentToken).put("href", href)
                .put("files", JSONArray(descriptors)).toString()
    }

    fun cancel() {
        pickerPreparation.cancel()
        revokeLease(preserveSelection = false)
    }

    private fun revokeLease(preserveSelection: Boolean) {
        if (leases.isEmpty()) return
        val previous = leases.values.toList()
        leases.clear()
        if (reading in previous) reading = null
        previous.forEach { it.reader.revoke() }
        if (ioDelegate.isInitialized()) io.execute { previous.forEach { it.reader.close() } }
        val token = JSONObject.quote(previous.first().documentToken)
        val command = if (preserveSelection) "suspend" else "cancel"
        webView.evaluateJavascript(
            "if(window.__elonChatGptDocumentToken===$token)window.__elonChatGptPrivateAttachmentSend?.$command();",
            null,
        )
    }

    fun dispose() {
        cancel()
        disposed = true
        main.removeCallbacksAndMessages(null)
        if (installed) WebViewCompat.removeWebMessageListener(webView, BRIDGE)
        installed = false
        // No executor is created unless a file was actually read.
        // Drain queued revocation cleanup; shutdownNow would discard those closes.
        if (ioDelegate.isInitialized()) io.shutdown()
    }

    private fun isCurrent(value: Lease): Boolean {
        val state = document()
        return !disposed && leases[value.id] === value && SystemClock.elapsedRealtime() < value.expiresAt &&
            state.adapterCurrent && state.documentToken == value.documentToken &&
            state.pageGeneration == value.generation && webView.url == value.href
    }

    private companion object {
        const val BRIDGE = "elonChatGptAttachmentSource"
        const val ORIGIN = "https://chatgpt.com"
        val REQUEST_ID = Regex("^attachment_[a-z0-9_]{1,70}$")
    }
}

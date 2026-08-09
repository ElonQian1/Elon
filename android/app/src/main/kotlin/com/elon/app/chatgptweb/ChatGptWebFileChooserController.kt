package com.elon.app.chatgptweb

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.MediaStore
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.MimeTypeMap
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import java.io.File

internal class ChatGptWebFileChooserController(
    private val activity: AppCompatActivity,
) {
    private var pendingCallback: ValueCallback<Array<Uri>>? = null
    private var pendingCaptureUri: Uri? = null
    private val launcher = activity.registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        val callback = pendingCallback ?: return@registerForActivityResult
        pendingCallback = null
        val captured = pendingCaptureUri
        pendingCaptureUri = null
        val selected = WebChromeClient.FileChooserParams.parseResult(result.resultCode, result.data)
        val captureResult: Array<Uri>? = if (
            captured != null && result.resultCode == Activity.RESULT_OK
        ) {
            arrayOf(captured)
        } else {
            null
        }
        callback.onReceiveValue(selected ?: captureResult)
    }

    fun show(
        webView: WebView,
        callback: ValueCallback<Array<Uri>>,
        params: WebChromeClient.FileChooserParams,
    ): Boolean {
        pendingCallback?.onReceiveValue(null)
        pendingCallback = null
        pendingCaptureUri = null
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
            callback.onReceiveValue(null)
            return true
        }
        pendingCallback = callback
        return runCatching {
            launcher.launch(createPickerIntent(params))
            true
        }.getOrElse {
            pendingCallback = null
            pendingCaptureUri = null
            callback.onReceiveValue(null)
            true
        }
    }

    fun dispose() {
        pendingCallback?.onReceiveValue(null)
        pendingCallback = null
        pendingCaptureUri = null
    }

    private fun createPickerIntent(params: WebChromeClient.FileChooserParams): Intent {
        val mimeTypes = sanitizeMimeTypes(params.acceptTypes)
        if (params.isCaptureEnabled && mimeTypes.any { it == "image/*" || it.startsWith("image/") }) {
            return createCameraIntent()
        }
        return Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            type = mimeTypes.singleOrNull() ?: "*/*"
            if (mimeTypes.size > 1) putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
            putExtra(
                Intent.EXTRA_ALLOW_MULTIPLE,
                params.mode == WebChromeClient.FileChooserParams.MODE_OPEN_MULTIPLE,
            )
        }
    }

    private fun createCameraIntent(): Intent {
        val directory = File(activity.cacheDir, "attachments").apply { mkdirs() }
        val target = File.createTempFile("chatgpt-capture-", ".jpg", directory)
        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.fileprovider",
            target,
        )
        pendingCaptureUri = uri
        return Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
            putExtra(MediaStore.EXTRA_OUTPUT, uri)
            clipData = android.content.ClipData.newRawUri("ChatGPT capture", uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        }
    }

    internal companion object {
        fun sanitizeMimeTypes(rawTypes: Array<String>): List<String> = rawTypes
            .asSequence()
            .flatMap { it.split(',').asSequence() }
            .map(String::trim)
            .map { value ->
                when {
                    value.startsWith('.') -> MimeTypeMap.getSingleton()
                        .getMimeTypeFromExtension(value.removePrefix(".").lowercase())
                    else -> value.substringBefore(';').lowercase()
                }
            }
            .filterNotNull()
            .filter { it == "*/*" || MIME_TYPE.matches(it) }
            .distinct()
            .take(MAX_MIME_TYPES)
            .toList()
            .ifEmpty { listOf("*/*") }

        private const val MAX_MIME_TYPES = 12
        private val MIME_TYPE = Regex("[a-z0-9!#$&^_.+-]+/[a-z0-9!#$&^_.+*-]+")
    }
}
